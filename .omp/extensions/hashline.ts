import type { ExtensionAPI, ExtensionContext, TruncationResult } from "@mariozechner/pi-coding-agent";
import {
    DEFAULT_MAX_BYTES,
    DEFAULT_MAX_LINES,
    formatSize,
    renderDiff,
    truncateHead,
    withFileMutationQueue,
} from "@mariozechner/pi-coding-agent";
import { Box, Container, Spacer, Text } from "@mariozechner/pi-tui";
import { Type, type Static } from "typebox";
import { constants } from "node:fs";
import { access, mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, normalize, resolve } from "node:path";

const HASHLINE_SEPARATOR = "\t";
const HASHLINE_ANCHOR_RE = /^\s*[>+\-*]*\s*(\d+)([a-z]{2})/u;
const HASHLINE_PREFIX_RE = /^\s*(?:>>>|>>)?\s*(?:[+*]\s*)?\d+[a-z]{2}(?:[:|]|\t| )/u;
const DISPLAY_HASHLINE_PREFIX_RE = /^(\s*(?:>>>|>>)?\s*(?:[+*\- ]\s*)?)(\d+)[a-z]{2}([:|]|\t| )/u;
const READ_DISPLAY_HASHLINE_PREFIX_RE = /^\d+[a-z]{2}(?:[:|]|\t| )/u;
const EMBEDDED_HASHLINE_RE = /\b(\d+)[a-z]{2}([:|])/gu;
const EXPECTED_ANCHOR_RE = /\b(expected\s+)(\d+)[a-z]{2}\b/giu;
const DIFF_PLUS_RE = /^[+](?![+])/u;
const STRUCTURAL_STRIP_RE = /[\s{}]/gu;
const SIGNIFICANT_RE = /[\p{L}\p{N}]/u;
const READ_PREVIEW_MAX_LINES = DEFAULT_MAX_LINES;
const READ_PREVIEW_MAX_BYTES = DEFAULT_MAX_BYTES;
const DIFF_PREVIEW_MAX_LINES = 500;
const DIFF_PREVIEW_MAX_BYTES = DEFAULT_MAX_BYTES;

type LineEnding = "\n" | "\r\n" | "\r";

type DecodedText = {
    bom: string;
    lineEnding: LineEnding;
    normalizedText: string;
};

type Anchor = {
    line: number;
    hash: string;
};

type ResolvedEdit =
    | { op: "replace_range"; pos: Anchor; end: Anchor; lines: string[] }
    | { op: "append_at"; pos: Anchor; lines: string[] }
    | { op: "prepend_at"; pos: Anchor; lines: string[] }
    | { op: "append_file"; lines: string[] }
    | { op: "prepend_file"; lines: string[] };

type HashMismatch = {
    line: number;
    expected: string;
    actual: string;
};

type DiffPreview = {
    text: string;
    addedLines: number;
    removedLines: number;
    firstChangedLine?: number;
    truncated?: TruncationResult;
};

const readParamsSchema = Type.Object(
    {
        path: Type.String({ description: "Text file path to read with LINE+ID hashline anchors" }),
        offset: Type.Optional(Type.Number({ description: "Line number to start reading from, 1-indexed" })),
        limit: Type.Optional(Type.Number({ description: "Maximum number of lines to read" })),
    },
    { additionalProperties: false },
);

const editLocSchema = Type.Union(
    [
        Type.Literal("append"),
        Type.Literal("prepend"),
        Type.Object({ append: Type.String({ description: "Full LINE+ID anchor to insert after" }) }),
        Type.Object({ prepend: Type.String({ description: "Full LINE+ID anchor to insert before" }) }),
        Type.Object({
            range: Type.Object(
                {
                    pos: Type.String({ description: "First full LINE+ID anchor in the inclusive replacement range" }),
                    end: Type.String({ description: "Last full LINE+ID anchor in the inclusive replacement range" }),
                },
                { additionalProperties: false },
            ),
        }),
    ],
    { description: "Edit location" },
);

const editEntrySchema = Type.Object(
    {
        loc: Type.Optional(editLocSchema),
        content: Type.Optional(
            Type.Union([
                Type.Array(Type.String(), { description: "Replacement/inserted lines, one string per logical line" }),
                Type.Null({ description: "Delete the targeted range" }),
            ]),
        ),
    },
    { additionalProperties: false },
);

const editParamsSchema = Type.Object(
    {
        path: Type.String({ description: "Text file path to edit" }),
        edits: Type.Array(editEntrySchema, { description: "Hashline edits to apply atomically" }),
    },
    { additionalProperties: false },
);

const writeParamsSchema = Type.Object(
    {
        path: Type.String({ description: "Path to the file to write (relative or absolute)" }),
        content: Type.String({ description: "Content to write to the file" }),
    },
    { additionalProperties: false },
);

type HashlineReadParams = Static<typeof readParamsSchema>;
type HashlineEditEntry = Static<typeof editEntrySchema>;
type HashlineEditParams = Static<typeof editParamsSchema>;
type HashlineWriteParams = Static<typeof writeParamsSchema>;

type HashlineReadDetails = {
    path: string;
    startLine: number;
    displayedLines: number;
    totalLines: number;
    truncation?: TruncationResult;
};

type HashlineEditDetails = {
    diff: string;
    firstChangedLine?: number;
    warnings?: string[];
    addedLines?: number;
    removedLines?: number;
};

type HashlineWriteDetails = {
    bytes: number;
};

type EditPreview =
    | {
          diff: string;
          firstChangedLine?: number;
          addedLines?: number;
          removedLines?: number;
      }
    | { error: string };

type EditRenderState = {
    callComponent?: HashlineEditCallRenderComponent;
};

type HashlineEditCallRenderComponent = Box & {
    preview?: EditPreview;
    previewArgsKey?: string;
    previewPending?: boolean;
    settledError?: boolean;
};


function normalizeToolPath(path: string): string {
    return path.startsWith("@") ? path.slice(1) : path;
}

function resolveToolPath(cwd: string, path: string): string {
    const cleaned = normalizeToolPath(path);
    return normalize(isAbsolute(cleaned) ? cleaned : resolve(cwd, cleaned));
}

function ordinalSuffix(lineNumber: number): string {
    const mod100 = lineNumber % 100;
    if (mod100 >= 11 && mod100 <= 13) return "th";

    switch (lineNumber % 10) {
        case 1:
            return "st";
        case 2:
            return "nd";
        case 3:
            return "rd";
        default:
            return "th";
    }
}

function bigramFromIndex(index: number): string {
    const normalizedIndex = index % (26 * 26);
    const first = Math.floor(normalizedIndex / 26);
    const second = normalizedIndex % 26;
    return String.fromCharCode(97 + first, 97 + second);
}

function fnv1a32(text: string, seed: number): number {
    let hash = (0x811c9dc5 ^ seed) >>> 0;
    for (let i = 0; i < text.length; i++) {
        hash ^= text.charCodeAt(i);
        hash = Math.imul(hash, 0x01000193) >>> 0;
    }
    return hash >>> 0;
}

export function computeLineHash(lineNumber: number, line: string): string {
    const normalizedLine = line.replace(/\r/gu, "").trimEnd();

    if (normalizedLine.replace(STRUCTURAL_STRIP_RE, "").length === 0) {
        return ordinalSuffix(lineNumber);
    }

    const seed = SIGNIFICANT_RE.test(normalizedLine) ? 0 : lineNumber;
    return bigramFromIndex(fnv1a32(normalizedLine, seed));
}

export function formatHashLine(lineNumber: number, line: string): string {
    return `${lineNumber}${computeLineHash(lineNumber, line)}${HASHLINE_SEPARATOR}${line}`;
}

function formatHashLines(lines: string[], startLine: number): string {
    return lines.map((line, index) => formatHashLine(startLine + index, line)).join("\n");
}

export function stripHashlineAnchorsForDisplay(text: string): string {
    return text
        .split("\n")
        .map((line) => line.replace(DISPLAY_HASHLINE_PREFIX_RE, "$1$2$3"))
        .join("\n")
        .replace(EMBEDDED_HASHLINE_RE, "$1$2")
        .replace(EXPECTED_ANCHOR_RE, "$1$2");
}

function stripHashlinePrefixesForDisplay(text: string): string {
    return text
        .split("\n")
        .map((line) => line.replace(READ_DISPLAY_HASHLINE_PREFIX_RE, ""))
        .join("\n");
}

type TextOnlyToolResult = {
    content?: Array<{ type: string; text?: string }>;
};

function collectTextContent(result: TextOnlyToolResult): string {
    return (result.content ?? [])
        .filter((part) => part.type === "text")
        .map((part) => part.text ?? "")
        .join("\n");
}

function trimTrailingEmptyLines(lines: string[]): string[] {
    let end = lines.length;
    while (end > 0 && lines[end - 1] === "") {
        end--;
    }
    return lines.slice(0, end);
}

function replaceTabs(text: string): string {
    return text.replace(/\t/gu, "   ");
}

function formatReadCall(args: Partial<HashlineReadParams>, theme: ExtensionContext["ui"]["theme"]): string {
    const path = typeof args.path === "string" && args.path.length > 0 ? args.path : "...";
    let pathDisplay = theme.fg("accent", path);
    if (args.offset !== undefined || args.limit !== undefined) {
        const startLine = args.offset ?? 1;
        const endLine = args.limit !== undefined ? startLine + args.limit - 1 : "";
        pathDisplay += theme.fg("warning", `:${startLine}${endLine ? `-${endLine}` : ""}`);
    }
    return `${theme.fg("toolTitle", theme.bold("read"))} ${pathDisplay}`;
}

function formatReadResultForDisplay(
    result: TextOnlyToolResult & { details?: HashlineReadDetails },
    options: { expanded?: boolean; isError?: boolean },
    theme: ExtensionContext["ui"]["theme"],
): string {
    const output = stripHashlinePrefixesForDisplay(collectTextContent(result));
    const lines = trimTrailingEmptyLines(output.split("\n")).map(replaceTabs);
    if (!options.expanded && !options.isError) {
        return "";
    }
    const displayLines = lines;
    let text = `\n${displayLines.map((line) => theme.fg("toolOutput", line)).join("\n")}`;
    const truncation = result.details?.truncation;
    if (truncation?.truncated) {
        text += theme.fg("warning", `\n[Truncated: showing ${truncation.outputLines} of ${truncation.totalLines} lines]`);
    }
    return text;
}

function createHashlineEditCallRenderComponent(): HashlineEditCallRenderComponent {
    return Object.assign(new Box(1, 1, (text) => text), {
        preview: undefined,
        previewArgsKey: undefined,
        previewPending: false,
        settledError: false,
    });
}

function getHashlineEditCallRenderComponent(
    state: EditRenderState,
    lastComponent: unknown,
): HashlineEditCallRenderComponent {
    if (lastComponent instanceof Box) {
        const component = lastComponent as HashlineEditCallRenderComponent;
        state.callComponent = component;
        return component;
    }
    if (state.callComponent) {
        return state.callComponent;
    }
    const component = createHashlineEditCallRenderComponent();
    state.callComponent = component;
    return component;
}

function getEditHeaderBg(preview: EditPreview | undefined, settledError: boolean | undefined, theme: ExtensionContext["ui"]["theme"]): (text: string) => string {
    if (preview) {
        if ("error" in preview) return (text: string) => theme.bg("toolErrorBg", text);
        return (text: string) => theme.bg("toolSuccessBg", text);
    }
    if (settledError) return (text: string) => theme.bg("toolErrorBg", text);
    return (text: string) => theme.bg("toolPendingBg", text);
}

function formatEditCall(args: Partial<HashlineEditParams>, theme: ExtensionContext["ui"]["theme"]): string {
    const path = typeof args.path === "string" && args.path.length > 0 ? args.path : "...";
    return `${theme.fg("toolTitle", theme.bold("edit"))} ${theme.fg("accent", path)}`;
}

function buildHashlineEditCallComponent(
    component: HashlineEditCallRenderComponent,
    args: Partial<HashlineEditParams>,
    theme: ExtensionContext["ui"]["theme"],
): HashlineEditCallRenderComponent {
    component.setBgFn(getEditHeaderBg(component.preview, component.settledError, theme));
    component.clear();
    component.addChild(new Text(formatEditCall(args, theme), 0, 0));
    if (!component.preview) {
        return component;
    }
    const body = "error" in component.preview
        ? theme.fg("error", stripHashlineAnchorsForDisplay(component.preview.error))
        : renderDiff(component.preview.diff, { filePath: args.path });
    component.addChild(new Spacer(1));
    component.addChild(new Text(body, 0, 0));
    return component;
}

function setEditPreview(component: HashlineEditCallRenderComponent, preview: EditPreview, argsKey?: string): boolean {
    const current = component.preview;
    const changed =
        current === undefined ||
        ("error" in current && "error" in preview
            ? current.error !== preview.error
            : "error" in current !== "error" in preview) ||
        (!("error" in current) &&
            !("error" in preview) &&
            (current.diff !== preview.diff || current.firstChangedLine !== preview.firstChangedLine));
    component.preview = preview;
    component.previewArgsKey = argsKey;
    component.previewPending = false;
    return changed;
}

function getRenderableHashlineEditInput(args: Partial<HashlineEditParams> | undefined): HashlineEditParams | null {
    if (!args || typeof args.path !== "string" || !Array.isArray(args.edits) || args.edits.length === 0) {
        return null;
    }
    return { path: args.path, edits: args.edits as HashlineEditEntry[] };
}

function formatWriteCall(args: Partial<HashlineWriteParams>, options: { expanded?: boolean }, theme: ExtensionContext["ui"]["theme"]): string {
    const path = typeof args.path === "string" && args.path.length > 0 ? args.path : "...";
    const content = typeof args.content === "string" ? args.content : "";
    let text = `${theme.fg("toolTitle", theme.bold("write"))} ${theme.fg("accent", path)}`;
    if (typeof args.content !== "string") {
        return `${text}\n\n${theme.fg("error", "[invalid content arg - expected string]")}`;
    }
    if (content.length === 0) {
        return text;
    }
    const lines = trimTrailingEmptyLines(content.split("\n")).map(replaceTabs);
    const maxLines = options.expanded ? lines.length : 10;
    const displayLines = lines.slice(0, maxLines);
    const remaining = lines.length - maxLines;
    text += `\n\n${displayLines.map((line) => theme.fg("toolOutput", line)).join("\n")}`;
    if (remaining > 0) {
        text += theme.fg("muted", `\n... (${remaining} more lines, ${lines.length} total, expand to show all)`);
    }
    return text;
}

function detectLineEnding(text: string): LineEnding {
    if (text.includes("\r\n")) return "\r\n";
    if (text.includes("\r")) return "\r";
    return "\n";
}

function normalizeToLf(text: string): string {
    return text.replace(/\r\n/gu, "\n").replace(/\r/gu, "\n");
}

function restoreLineEndings(text: string, lineEnding: LineEnding): string {
    return lineEnding === "\n" ? text : text.replace(/\n/gu, lineEnding);
}

function isNotFoundError(error: unknown): boolean {
    return !!error && typeof error === "object" && (error as { code?: unknown }).code === "ENOENT";
}

function decodeUtf8Text(buffer: Buffer, path: string): DecodedText {
    const textWithBom = buffer.toString("utf8");
    if (textWithBom.includes("\u0000")) {
        throw new Error(`Hashline tools support UTF-8 text files only; ${path} appears to contain NUL bytes.`);
    }

    const bom = textWithBom.startsWith("\uFEFF") ? "\uFEFF" : "";
    const text = bom ? textWithBom.slice(1) : textWithBom;
    const lineEnding = detectLineEnding(text);
    return {
        bom,
        lineEnding,
        normalizedText: normalizeToLf(text),
    };
}

function validateLineWindow(offset: number | undefined, limit: number | undefined): { startIndex: number; limit?: number } {
    if (offset !== undefined && (!Number.isFinite(offset) || offset < 1)) {
        throw new Error(`offset must be a positive 1-indexed line number, got ${offset}.`);
    }
    if (limit !== undefined && (!Number.isFinite(limit) || limit < 1)) {
        throw new Error(`limit must be a positive line count, got ${limit}.`);
    }

    return {
        startIndex: offset === undefined ? 0 : Math.trunc(offset) - 1,
        limit: limit === undefined ? undefined : Math.trunc(limit),
    };
}

function formatFullAnchorRequirement(raw?: string): string {
    const received = raw === undefined ? "" : ` Received ${JSON.stringify(raw)}.`;
    const suffix = typeof raw === "string" ? raw.trim() : "";
    const suffixHint = /^[a-z]{2}$/iu.test(suffix)
        ? ` It looks like only the 2-letter suffix was supplied; copy the full anchor including the line number.`
        : "";
    return `the full anchor exactly as shown by read (line number + 2-letter suffix, for example "160sr").${received}${suffixHint}`;
}

function parseAnchor(raw: string): Anchor {
    const match = raw.match(HASHLINE_ANCHOR_RE);
    if (!match) {
        throw new Error(`Invalid line reference. Expected ${formatFullAnchorRequirement(raw)}`);
    }

    const line = Number.parseInt(match[1], 10);
    if (!Number.isInteger(line) || line < 1) {
        throw new Error(`Line number must be >= 1 in anchor ${JSON.stringify(raw)}.`);
    }
    return { line, hash: match[2].toLowerCase() };
}

function stripHashlinePrefix(line: string): string {
    let current = line;
    let previous: string;
    do {
        previous = current;
        current = current.replace(HASHLINE_PREFIX_RE, "");
    } while (current !== previous);
    return current;
}

function normalizeContentLines(content: string[] | null | undefined): string[] {
    if (content == null) return [];

    const lines = content.flatMap((line) => {
        const withoutCarriageReturns = line.replace(/\r/gu, "");
        const trimmedTrailingNewline = withoutCarriageReturns.endsWith("\n")
            ? withoutCarriageReturns.slice(0, -1)
            : withoutCarriageReturns;
        return trimmedTrailingNewline.split("\n");
    });

    const meaningfulLines = lines.filter((line) => line.length > 0);
    if (meaningfulLines.length === 0) {
        return lines;
    }

    const allHashPrefixed = meaningfulLines.every((line) => HASHLINE_PREFIX_RE.test(line));
    if (allHashPrefixed) {
        return lines.map((line) => (line.length === 0 ? line : stripHashlinePrefix(line)));
    }

    const diffPlusCount = meaningfulLines.filter((line) => DIFF_PLUS_RE.test(line)).length;
    if (diffPlusCount >= meaningfulLines.length * 0.5) {
        return lines.map((line) => line.replace(DIFF_PLUS_RE, ""));
    }

    return lines;
}

function ensureInsertedContent(edit: ResolvedEdit): void {
    if ((edit.op === "append_at" || edit.op === "prepend_at" || edit.op === "append_file" || edit.op === "prepend_file") && edit.lines.length === 0) {
        edit.lines = [""];
    }
}

function resolveEdit(edit: HashlineEditEntry, index: number): ResolvedEdit {
    const lines = normalizeContentLines(edit.content);
    const loc = edit.loc;

    if (loc === "append") return { op: "append_file", lines };
    if (loc === "prepend") return { op: "prepend_file", lines };
    if (!loc || typeof loc !== "object") {
        throw new Error(`Edit ${index} has invalid loc. Expected "append", "prepend", {append}, {prepend}, or {range}.`);
    }

    if ("append" in loc) return { op: "append_at", pos: parseAnchor(loc.append), lines };
    if ("prepend" in loc) return { op: "prepend_at", pos: parseAnchor(loc.prepend), lines };
    if ("range" in loc) {
        const pos = parseAnchor(loc.range.pos);
        const end = parseAnchor(loc.range.end);
        if (pos.line > end.line) {
            throw new Error(`Edit ${index} range start line ${pos.line} must be <= end line ${end.line}.`);
        }
        return { op: "replace_range", pos, end, lines };
    }

    throw new Error(`Edit ${index} has unknown loc shape.`);
}

function validateAnchor(anchor: Anchor, fileLines: string[], mismatches: HashMismatch[]): void {
    if (anchor.line < 1 || anchor.line > fileLines.length) {
        throw new Error(`Line ${anchor.line} does not exist (file has ${fileLines.length} lines). Re-read the file before editing.`);
    }

    const actual = computeLineHash(anchor.line, fileLines[anchor.line - 1]);
    if (actual !== anchor.hash) {
        mismatches.push({ line: anchor.line, expected: anchor.hash, actual });
    }
}

function formatMismatchError(mismatches: HashMismatch[], fileLines: string[]): string {
    const mismatchLines = new Set(mismatches.map((mismatch) => mismatch.line));
    const displayLines = new Set<number>();
    for (const mismatch of mismatches) {
        for (let line = Math.max(1, mismatch.line - 2); line <= Math.min(fileLines.length, mismatch.line + 2); line++) {
            displayLines.add(line);
        }
    }

    const sorted = [...displayLines].sort((a, b) => a - b);
    const lines = [
        `Edit rejected: ${mismatches.length} anchor${mismatches.length === 1 ? " is" : "s are"} stale or mismatched.`,
        "The edit was NOT applied. Re-read or use the updated anchors below, then issue another edit call.",
        "",
    ];

    let previous = -1;
    for (const lineNumber of sorted) {
        if (previous !== -1 && lineNumber > previous + 1) {
            lines.push("...");
        }
        previous = lineNumber;
        const marker = mismatchLines.has(lineNumber) ? "*" : " ";
        const mismatch = mismatches.find((item) => item.line === lineNumber);
        const expected = mismatch ? ` (expected ${lineNumber}${mismatch.expected})` : "";
        lines.push(`${marker}${formatHashLine(lineNumber, fileLines[lineNumber - 1])}${expected}`);
    }

    return lines.join("\n");
}

function validateEditAnchors(edits: ResolvedEdit[], fileLines: string[]): void {
    const mismatches: HashMismatch[] = [];
    for (const edit of edits) {
        switch (edit.op) {
            case "replace_range":
                validateAnchor(edit.pos, fileLines, mismatches);
                validateAnchor(edit.end, fileLines, mismatches);
                break;
            case "append_at":
            case "prepend_at":
                validateAnchor(edit.pos, fileLines, mismatches);
                ensureInsertedContent(edit);
                break;
            case "append_file":
            case "prepend_file":
                ensureInsertedContent(edit);
                break;
        }
    }

    if (mismatches.length > 0) {
        throw new Error(formatMismatchError(mismatches, fileLines));
    }
}

function collectBoundaryWarnings(edits: ResolvedEdit[], originalLines: string[]): string[] {
    const warnings: string[] = [];
    for (const edit of edits) {
        if (edit.op !== "replace_range" || edit.lines.length === 0) {
            continue;
        }

        const nextLineIndex = edit.end.line;
        if (nextLineIndex >= originalLines.length) {
            continue;
        }

        const lastInserted = edit.lines[edit.lines.length - 1].trim();
        const nextSurviving = originalLines[nextLineIndex].trim();
        if (lastInserted.length > 0 && lastInserted === nextSurviving) {
            warnings.push(
                `Possible boundary duplication: replacement ends with ${JSON.stringify(lastInserted)}, which matches the next surviving line ${formatHashLine(
                    edit.end.line + 1,
                    originalLines[nextLineIndex],
                )}. If you intended to replace the whole block, extend the range end to that anchor.`,
            );
        }
    }
    return warnings;
}

function dedupeEdits(edits: ResolvedEdit[]): ResolvedEdit[] {
    const seen = new Set<string>();
    const result: ResolvedEdit[] = [];
    for (const edit of edits) {
        const key = JSON.stringify(edit);
        if (seen.has(key)) continue;
        seen.add(key);
        result.push(edit);
    }
    return result;
}

function editSortKey(edit: ResolvedEdit, fileLineCount: number): { line: number; precedence: number } {
    switch (edit.op) {
        case "replace_range":
            return { line: edit.end.line, precedence: 0 };
        case "append_at":
            return { line: edit.pos.line, precedence: 1 };
        case "prepend_at":
            return { line: edit.pos.line, precedence: 2 };
        case "append_file":
            return { line: fileLineCount + 1, precedence: 1 };
        case "prepend_file":
            return { line: 0, precedence: 2 };
    }
}

function applyResolvedEdits(originalText: string, edits: ResolvedEdit[]): { text: string; firstChangedLine?: number; warnings: string[] } {
    const fileLines = originalText.split("\n");
    const originalLines = [...fileLines];
    const warnings = collectBoundaryWarnings(edits, originalLines);
    let firstChangedLine: number | undefined;
    const endedWithNewline = originalText.endsWith("\n");

    validateEditAnchors(edits, fileLines);

    const sorted = dedupeEdits(edits)
        .map((edit, index) => ({ edit, index, ...editSortKey(edit, fileLines.length) }))
        .sort((a, b) => b.line - a.line || a.precedence - b.precedence || a.index - b.index);

    const trackChanged = (line: number) => {
        if (firstChangedLine === undefined || line < firstChangedLine) {
            firstChangedLine = line;
        }
    };

    for (const { edit } of sorted) {
        switch (edit.op) {
            case "replace_range": {
                const count = edit.end.line - edit.pos.line + 1;
                fileLines.splice(edit.pos.line - 1, count, ...edit.lines);
                trackChanged(edit.pos.line);
                break;
            }
            case "append_at":
                fileLines.splice(edit.pos.line, 0, ...edit.lines);
                trackChanged(edit.pos.line + 1);
                break;
            case "prepend_at":
                fileLines.splice(edit.pos.line - 1, 0, ...edit.lines);
                trackChanged(edit.pos.line);
                break;
            case "append_file": {
                if (fileLines.length === 1 && fileLines[0] === "") {
                    fileLines.splice(0, 1, ...edit.lines);
                    trackChanged(1);
                    break;
                }
                const insertIndex = endedWithNewline && fileLines[fileLines.length - 1] === "" ? fileLines.length - 1 : fileLines.length;
                fileLines.splice(insertIndex, 0, ...edit.lines);
                trackChanged(insertIndex + 1);
                break;
            }
            case "prepend_file":
                if (fileLines.length === 1 && fileLines[0] === "") {
                    fileLines.splice(0, 1, ...edit.lines);
                } else {
                    fileLines.splice(0, 0, ...edit.lines);
                }
                trackChanged(1);
                break;
        }
    }

    return { text: fileLines.join("\n"), firstChangedLine, warnings };
}

type LineDiffPart = {
    value: string;
    added?: boolean;
    removed?: boolean;
};

function splitChangedPartLines(value: string): string[] {
    const lines = value.split("\n");
    if (lines[lines.length - 1] === "") {
        lines.pop();
    }
    return lines;
}

function splitDiffInputLines(value: string): string[] {
    const lines = value.split("\n");
    if (lines[lines.length - 1] === "") {
        lines.pop();
    }
    return lines;
}

function pushLineDiffPart(parts: LineDiffPart[], lines: string[], kind?: "added" | "removed"): void {
    if (lines.length === 0) return;
    const value = `${lines.join("\n")}\n`;
    const previous = parts[parts.length - 1];
    const isAdded = kind === "added";
    const isRemoved = kind === "removed";
    if (previous && !!previous.added === isAdded && !!previous.removed === isRemoved) {
        previous.value += value;
        return;
    }
    parts.push({ value, ...(isAdded ? { added: true } : {}), ...(isRemoved ? { removed: true } : {}) });
}

function diffSmallLineRange(oldLines: string[], newLines: string[]): LineDiffPart[] {
    const rows: Uint32Array[] = Array.from({ length: oldLines.length + 1 }, () => new Uint32Array(newLines.length + 1));
    for (let i = oldLines.length - 1; i >= 0; i--) {
        for (let j = newLines.length - 1; j >= 0; j--) {
            rows[i][j] = oldLines[i] === newLines[j]
                ? rows[i + 1][j + 1] + 1
                : Math.max(rows[i + 1][j], rows[i][j + 1]);
        }
    }

    const parts: LineDiffPart[] = [];
    let i = 0;
    let j = 0;
    while (i < oldLines.length && j < newLines.length) {
        if (oldLines[i] === newLines[j]) {
            pushLineDiffPart(parts, [oldLines[i]]);
            i++;
            j++;
        } else if (rows[i + 1][j] >= rows[i][j + 1]) {
            pushLineDiffPart(parts, [oldLines[i]], "removed");
            i++;
        } else {
            pushLineDiffPart(parts, [newLines[j]], "added");
            j++;
        }
    }
    if (i < oldLines.length) pushLineDiffPart(parts, oldLines.slice(i), "removed");
    if (j < newLines.length) pushLineDiffPart(parts, newLines.slice(j), "added");
    return parts;
}

function diffLinesLocal(beforeText: string, afterText: string): LineDiffPart[] {
    const oldLines = splitDiffInputLines(beforeText);
    const newLines = splitDiffInputLines(afterText);
    let prefix = 0;
    while (prefix < oldLines.length && prefix < newLines.length && oldLines[prefix] === newLines[prefix]) {
        prefix++;
    }

    let suffix = 0;
    while (
        suffix < oldLines.length - prefix &&
        suffix < newLines.length - prefix &&
        oldLines[oldLines.length - 1 - suffix] === newLines[newLines.length - 1 - suffix]
    ) {
        suffix++;
    }

    const oldMiddle = oldLines.slice(prefix, oldLines.length - suffix);
    const newMiddle = newLines.slice(prefix, newLines.length - suffix);
    const parts: LineDiffPart[] = [];
    pushLineDiffPart(parts, oldLines.slice(0, prefix));

    if (oldMiddle.length * newMiddle.length <= 250_000) {
        for (const part of diffSmallLineRange(oldMiddle, newMiddle)) {
            pushLineDiffPart(
                parts,
                splitChangedPartLines(part.value),
                part.added ? "added" : part.removed ? "removed" : undefined,
            );
        }
    } else {
        pushLineDiffPart(parts, oldMiddle, "removed");
        pushLineDiffPart(parts, newMiddle, "added");
    }

    pushLineDiffPart(parts, oldLines.slice(oldLines.length - suffix));
    return parts;
}

function buildDiffPreview(_path: string, beforeText: string, afterText: string): DiffPreview {
    const parts = diffLinesLocal(beforeText, afterText);
    const output: string[] = [];
    const oldLines = beforeText.split("\n");
    const newLines = afterText.split("\n");
    const maxLineNum = Math.max(oldLines.length, newLines.length);
    const lineNumWidth = String(maxLineNum).length;
    let oldLineNum = 1;
    let newLineNum = 1;
    let firstChangedLine: number | undefined;
    let lastWasChange = false;
    let addedLines = 0;
    let removedLines = 0;
    const contextLines = 4;

    for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const raw = splitChangedPartLines(part.value);

        if (part.added || part.removed) {
            if (firstChangedLine === undefined) {
                firstChangedLine = newLineNum;
            }
            for (const line of raw) {
                if (part.added) {
                    const lineNum = String(newLineNum).padStart(lineNumWidth, " ");
                    output.push(`+${lineNum} ${line}`);
                    newLineNum++;
                    addedLines++;
                } else {
                    const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
                    output.push(`-${lineNum} ${line}`);
                    oldLineNum++;
                    removedLines++;
                }
            }
            lastWasChange = true;
            continue;
        }

        const nextPartIsChange = i < parts.length - 1 && (parts[i + 1].added || parts[i + 1].removed);
        const hasLeadingChange = lastWasChange;
        const hasTrailingChange = nextPartIsChange;

        if (hasLeadingChange && hasTrailingChange) {
            if (raw.length <= contextLines * 2) {
                for (const line of raw) {
                    const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
                    output.push(` ${lineNum} ${line}`);
                    oldLineNum++;
                    newLineNum++;
                }
            } else {
                const leadingLines = raw.slice(0, contextLines);
                const trailingLines = raw.slice(raw.length - contextLines);
                const skippedLines = raw.length - leadingLines.length - trailingLines.length;
                for (const line of leadingLines) {
                    const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
                    output.push(` ${lineNum} ${line}`);
                    oldLineNum++;
                    newLineNum++;
                }
                output.push(` ${"".padStart(lineNumWidth, " ")} ...`);
                oldLineNum += skippedLines;
                newLineNum += skippedLines;
                for (const line of trailingLines) {
                    const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
                    output.push(` ${lineNum} ${line}`);
                    oldLineNum++;
                    newLineNum++;
                }
            }
        } else if (hasLeadingChange) {
            const shownLines = raw.slice(0, contextLines);
            const skippedLines = raw.length - shownLines.length;
            for (const line of shownLines) {
                const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
                output.push(` ${lineNum} ${line}`);
                oldLineNum++;
                newLineNum++;
            }
            if (skippedLines > 0) {
                output.push(` ${"".padStart(lineNumWidth, " ")} ...`);
                oldLineNum += skippedLines;
                newLineNum += skippedLines;
            }
        } else if (hasTrailingChange) {
            const skippedLines = Math.max(0, raw.length - contextLines);
            if (skippedLines > 0) {
                output.push(` ${"".padStart(lineNumWidth, " ")} ...`);
                oldLineNum += skippedLines;
                newLineNum += skippedLines;
            }
            for (const line of raw.slice(skippedLines)) {
                const lineNum = String(oldLineNum).padStart(lineNumWidth, " ");
                output.push(` ${lineNum} ${line}`);
                oldLineNum++;
                newLineNum++;
            }
        } else {
            oldLineNum += raw.length;
            newLineNum += raw.length;
        }

        lastWasChange = false;
    }

    let text = output.join("\n");
    const truncation = truncateHead(text, { maxLines: DIFF_PREVIEW_MAX_LINES, maxBytes: DIFF_PREVIEW_MAX_BYTES });
    if (truncation.truncated) {
        text = `${truncation.content}\n\n[Diff preview truncated: showing ${truncation.outputLines} of ${truncation.totalLines} lines (${formatSize(
            truncation.outputBytes,
        )} of ${formatSize(truncation.totalBytes)}).]`;
    }

    return { text, addedLines, removedLines, firstChangedLine, truncated: truncation.truncated ? truncation : undefined };
}

export async function executeHashlineRead(params: HashlineReadParams, cwd: string): Promise<{ text: string; details: HashlineReadDetails }> {
    const absolutePath = resolveToolPath(cwd, params.path);
    const { startIndex, limit } = validateLineWindow(params.offset, params.limit);

    await access(absolutePath, constants.R_OK);
    const decoded = decodeUtf8Text(await readFile(absolutePath), params.path);
    const allLines = decoded.normalizedText.split("\n");

    if (startIndex >= allLines.length) {
        return {
            text: `Line ${startIndex + 1} is beyond end of file (${allLines.length} lines total). Use offset=1 to read from the start.`,
            details: {
                path: params.path,
                startLine: startIndex + 1,
                displayedLines: 0,
                totalLines: allLines.length,
            },
        };
    }

    const endIndex = limit === undefined ? allLines.length : Math.min(allLines.length, startIndex + limit);
    const selected = allLines.slice(startIndex, endIndex);
    const formatted = formatHashLines(selected, startIndex + 1);
    const truncation = truncateHead(formatted, { maxLines: READ_PREVIEW_MAX_LINES, maxBytes: READ_PREVIEW_MAX_BYTES });
    let text = truncation.content;

    if (truncation.firstLineExceedsLimit || text.length === 0) {
        text = `[Line ${startIndex + 1} exceeds the ${formatSize(READ_PREVIEW_MAX_BYTES)} hashline output limit. Hashline output requires complete lines.]`;
    }

    if (truncation.truncated) {
        text += `\n\n[Output truncated: showing ${truncation.outputLines} of ${truncation.totalLines} formatted lines (${formatSize(
            truncation.outputBytes,
        )} of ${formatSize(truncation.totalBytes)}). Use a narrower offset/limit window.]`;
    } else if (endIndex < allLines.length) {
        text += `\n\n[${allLines.length - endIndex} more lines in file. Use offset=${endIndex + 1} to continue.]`;
    }

    return {
        text,
        details: {
            path: params.path,
            startLine: startIndex + 1,
            displayedLines: truncation.truncated || truncation.firstLineExceedsLimit ? truncation.outputLines : selected.length,
            totalLines: allLines.length,
            truncation: truncation.truncated ? truncation : undefined,
        },
    };
}

export async function computeHashlineEditDiff(params: HashlineEditParams, cwd: string): Promise<EditPreview> {
    try {
        const absolutePath = resolveToolPath(cwd, params.path);
        const resolvedEdits = params.edits.map(resolveEdit);
        let source: Buffer | undefined;
        try {
            source = await readFile(absolutePath);
        } catch (error) {
            if (!isNotFoundError(error)) {
                const message = error instanceof Error ? error.message : String(error);
                return { error: `Unable to read ${params.path}: ${message}` };
            }
        }

        if (!source) {
            const lines: string[] = [];
            for (const edit of resolvedEdits) {
                ensureInsertedContent(edit);
                if (edit.op === "append_file") {
                    lines.push(...edit.lines);
                } else if (edit.op === "prepend_file") {
                    lines.unshift(...edit.lines);
                } else {
                    return { error: `File not found: ${params.path}. New files can only use loc="append" or loc="prepend".` };
                }
            }
            const diff = buildDiffPreview(params.path, "", lines.join("\n"));
            return {
                diff: diff.text,
                firstChangedLine: diff.firstChangedLine ?? 1,
                addedLines: diff.addedLines,
                removedLines: diff.removedLines,
            };
        }

        const beforeDecoded = decodeUtf8Text(source, params.path);
        const beforeText = beforeDecoded.normalizedText;
        const applied = applyResolvedEdits(beforeText, resolvedEdits);
        if (beforeText === applied.text) {
            return { error: `No changes made to ${params.path}. The edits produced identical content.` };
        }
        const diff = buildDiffPreview(params.path, beforeText, applied.text);
        return {
            diff: diff.text,
            firstChangedLine: applied.firstChangedLine ?? diff.firstChangedLine,
            addedLines: diff.addedLines,
            removedLines: diff.removedLines,
        };
    } catch (error) {
        return { error: error instanceof Error ? error.message : String(error) };
    }
}

export async function executeHashlineEdit(params: HashlineEditParams, cwd: string, signal?: AbortSignal): Promise<{ text: string; details: HashlineEditDetails }> {
    if (params.edits.length === 0) {
        throw new Error("edit requires at least one hashline edit entry.");
    }

    const absolutePath = resolveToolPath(cwd, params.path);
    const resolvedEdits = params.edits.map(resolveEdit);

    return withFileMutationQueue(absolutePath, async () => {
        if (signal?.aborted) throw new Error("edit cancelled.");

        let source: Buffer | undefined;
        try {
            source = await readFile(absolutePath);
            await access(absolutePath, constants.W_OK);
        } catch (error) {
            if (!isNotFoundError(error)) {
                const message = error instanceof Error ? error.message : String(error);
                throw new Error(`Unable to access ${params.path}: ${message}`);
            }
        }

        if (!source) {
            const lines: string[] = [];
            for (const edit of resolvedEdits) {
                ensureInsertedContent(edit);
                if (edit.op === "append_file") {
                    lines.push(...edit.lines);
                } else if (edit.op === "prepend_file") {
                    lines.unshift(...edit.lines);
                } else {
                    throw new Error(`File not found: ${params.path}. New files can only use loc="append" or loc="prepend".`);
                }
            }
            await mkdir(dirname(absolutePath), { recursive: true });
            await writeFile(absolutePath, lines.join("\n"), "utf8");
            const diff = buildDiffPreview(params.path, "", lines.join("\n"));
            return {
                text: `Created ${params.path}\nChanges: +${diff.addedLines} -${diff.removedLines}`,
                details: {
                    diff: diff.text,
                    firstChangedLine: diff.firstChangedLine ?? 1,
                    addedLines: diff.addedLines,
                    removedLines: diff.removedLines,
                },
            };
        }

        const beforeDecoded = decodeUtf8Text(source, params.path);
        const beforeText = beforeDecoded.normalizedText;
        const applied = applyResolvedEdits(beforeText, resolvedEdits);

        if (beforeText === applied.text) {
            throw new Error(`No changes made to ${params.path}. The edits produced identical content.`);
        }

        if (signal?.aborted) throw new Error("edit cancelled.");

        const finalContent = beforeDecoded.bom + restoreLineEndings(applied.text, beforeDecoded.lineEnding);
        await writeFile(absolutePath, finalContent, "utf8");

        const diff = buildDiffPreview(params.path, beforeText, applied.text);
        const firstChangedLine = applied.firstChangedLine ?? diff.firstChangedLine;
        const warningsBlock = applied.warnings.length > 0 ? `\n\nWarnings:\n${applied.warnings.join("\n")}` : "";

        return {
            text: `Updated ${params.path}\nChanges: +${diff.addedLines} -${diff.removedLines}${warningsBlock}`,
            details: {
                diff: diff.text,
                firstChangedLine,
                warnings: applied.warnings.length > 0 ? applied.warnings : undefined,
                addedLines: diff.addedLines,
                removedLines: diff.removedLines,
            },
        };
    });
}

export async function executeHashlineWrite(params: HashlineWriteParams, cwd: string, signal?: AbortSignal): Promise<{ text: string; details: HashlineWriteDetails }> {
    const absolutePath = resolveToolPath(cwd, params.path);
    return withFileMutationQueue(absolutePath, async () => {
        if (signal?.aborted) throw new Error("write cancelled.");
        await mkdir(dirname(absolutePath), { recursive: true });
        await writeFile(absolutePath, params.content, "utf8");
        return {
            text: `Successfully wrote ${params.content.length} bytes to ${params.path}`,
            details: { bytes: params.content.length },
        };
    });
}

export default function (pi: ExtensionAPI) {
    pi.registerTool({
        name: "read",
        label: "read",
        description: `Read a UTF-8 text file with hidden LINE+ID anchors for the edit tool. Model-visible output is truncated to ${DEFAULT_MAX_LINES} lines or ${formatSize(
            DEFAULT_MAX_BYTES,
        )}. Use the full anchors (for example "160sr") with edit.`,
        promptSnippet: "Read UTF-8 text files with LINE+ID anchors for edit.",
        promptGuidelines: [
            "Use read to examine text files before edit; copy the full LINE+ID anchors exactly, including the line number.",
            "Read prefixes each model-visible line with a LINE+ID anchor followed by a tab; the UI hides read content until tool output is expanded.",
        ],
        parameters: readParamsSchema,
        executionMode: "sequential",
        async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
            const result = await executeHashlineRead(params, ctx.cwd);
            return {
                content: [{ type: "text", text: result.text }],
                details: result.details,
            };
        },
        renderCall(args, theme, context) {
            const text = context.lastComponent instanceof Text ? context.lastComponent : new Text("", 0, 0);
            text.setText(formatReadCall(args, theme));
            return text;
        },
        renderResult(result, options, theme, context) {
            const text = context.lastComponent instanceof Text ? context.lastComponent : new Text("", 0, 0);
            text.setText(formatReadResultForDisplay(result, { expanded: options.expanded, isError: context.isError }, theme));
            return text;
        },
    });

    pi.registerTool({
        name: "edit",
        label: "edit",
        description:
            "Apply precise UTF-8 text edits using full LINE+ID anchors from read output. Supports loc=\"append\", loc=\"prepend\", {append}, {prepend}, and inclusive {range:{pos,end}} with content as string[] or null for deletion.",
        promptSnippet: "Apply anchor-based file edits using LINE+ID anchors from read.",
        promptGuidelines: [
            "Use edit for anchored line edits after read, especially when exact oldText replacement would be large or fragile.",
            "For edit, range endpoints are inclusive; set pos and end to the same full anchor for a single-line replacement.",
            "For edit, if replacement content includes a closing delimiter that matches the next surviving line, extend the range to include that line to avoid duplicate delimiters.",
            "Do not use edit for unrelated reformatting; make the minimum exact edit.",
        ],
        parameters: editParamsSchema,
        executionMode: "sequential",
        async execute(_toolCallId, params, signal, _onUpdate, ctx) {
            const result = await executeHashlineEdit(params, ctx.cwd, signal);
            return {
                content: [{ type: "text", text: result.text }],
                details: result.details,
            };
        },
        renderShell: "self",
        renderCall(args, theme, context) {
            const state = (context.state ?? {}) as EditRenderState;
            const component = getHashlineEditCallRenderComponent(state, context.lastComponent);
            const previewInput = getRenderableHashlineEditInput(args);
            const argsKey = previewInput ? JSON.stringify(previewInput) : undefined;
            if (component.previewArgsKey !== argsKey) {
                component.preview = undefined;
                component.previewArgsKey = argsKey;
                component.previewPending = false;
                component.settledError = false;
            }
            if (context.argsComplete && previewInput && !component.preview && !component.previewPending) {
                component.previewPending = true;
                const requestKey = argsKey;
                void computeHashlineEditDiff(previewInput, context.cwd).then((preview) => {
                    if (component.previewArgsKey === requestKey) {
                        setEditPreview(component, preview, requestKey);
                        context.invalidate();
                    }
                });
            }
            return buildHashlineEditCallComponent(component, args, theme);
        },
        renderResult(result, _options, theme, context) {
            const state = (context.state ?? {}) as EditRenderState;
            const callComponent = state.callComponent;
            const details = result.details as HashlineEditDetails | undefined;
            const diff = !context.isError ? details?.diff : undefined;
            const argsKey = getRenderableHashlineEditInput(context.args) ? JSON.stringify(getRenderableHashlineEditInput(context.args)) : undefined;
            let callComponentUpdated = false;
            if (callComponent) {
                if (diff) {
                    callComponentUpdated = setEditPreview(
                        callComponent,
                        {
                            diff,
                            firstChangedLine: details?.firstChangedLine,
                            addedLines: details?.addedLines,
                            removedLines: details?.removedLines,
                        },
                        argsKey,
                    );
                }
                if (callComponent.settledError !== context.isError) {
                    callComponent.settledError = context.isError;
                    callComponentUpdated = true;
                }
                if (callComponentUpdated) {
                    buildHashlineEditCallComponent(callComponent, context.args, theme);
                }
            }

            const component = context.lastComponent instanceof Container ? context.lastComponent : new Container();
            component.clear();

            if (context.isError) {
                component.addChild(new Text(theme.fg("error", stripHashlineAnchorsForDisplay(collectTextContent(result))), 0, 0));
                return component;
            }

            if (!callComponent && diff) {
                component.addChild(new Spacer(1));
                component.addChild(new Text(renderDiff(diff, { filePath: typeof context.args.path === "string" ? context.args.path : undefined }), 1, 0));
            }
            if (details?.warnings?.length) {
                component.addChild(new Spacer(1));
                component.addChild(new Text(theme.fg("warning", stripHashlineAnchorsForDisplay(details.warnings.join("\n"))), 1, 0));
            }
            return component;
        },
    });

    pi.registerTool({
        name: "write",
        label: "write",
        description: "Write content to a UTF-8 text file. Creates the file if it doesn't exist, overwrites if it does. Automatically creates parent directories.",
        promptSnippet: "Create or overwrite UTF-8 text files",
        promptGuidelines: ["Use write only for new files or complete rewrites; use edit for anchored changes to existing text files."],
        parameters: writeParamsSchema,
        executionMode: "sequential",
        async execute(_toolCallId, params, signal, _onUpdate, ctx) {
            const result = await executeHashlineWrite(params, ctx.cwd, signal);
            return {
                content: [{ type: "text", text: result.text }],
                details: result.details,
            };
        },
        renderCall(args, theme, context) {
            const text = context.lastComponent instanceof Text ? context.lastComponent : new Text("", 0, 0);
            text.setText(formatWriteCall(args, { expanded: context.expanded }, theme));
            return text;
        },
        renderResult(result, _options, theme, context) {
            const component = context.lastComponent instanceof Container ? context.lastComponent : new Container();
            component.clear();
            if (context.isError) {
                component.addChild(new Text(theme.fg("error", collectTextContent(result)), 0, 0));
            }
            return component;
        },
    });

    pi.registerCommand("hashline-help", {
        description: "Show read/edit hashline usage.",
        handler: async (_args, ctx) => {
            ctx.ui.notify(
                'Use read on a text file, then edit with loc ranges such as {range:{pos:"12ab",end:"14cd"}} and content as string[]. Use content:null to delete.',
                "info",
            );
        },
    });
}
