import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const sourcePath = path.resolve(
  here,
  "../../../../results/tonic/2026-04-01-loop2/matrix_summary.csv",
);

process.stdout.write(await fs.readFile(sourcePath));
