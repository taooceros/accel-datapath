/*
 * DSA Launcher - launches binaries with cap_sys_rawio capability
 *
 * Build: gcc -o dsa_launcher dsa_launcher.c -lcap
 * Setup: sudo setcap cap_sys_rawio+eip dsa_launcher
 * Usage: ./dsa_launcher ./build/linux/x86_64/releasedbg/dsa_benchmark [args...]
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/capability.h>
#include <sys/prctl.h>
#include <errno.h>

int main(int argc, char *argv[]) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <program> [args...]\n", argv[0]);
        fprintf(stderr, "Launches a program with cap_sys_rawio capability for DSA access.\n");
        return 1;
    }

    // Get current capabilities
    cap_t caps = cap_get_proc();
    if (caps == NULL) {
        perror("cap_get_proc");
        return 1;
    }

    // Check if we have cap_sys_rawio
    cap_flag_value_t has_rawio;
    if (cap_get_flag(caps, CAP_SYS_RAWIO, CAP_EFFECTIVE, &has_rawio) != 0) {
        perror("cap_get_flag");
        cap_free(caps);
        return 1;
    }

    if (has_rawio != CAP_SET) {
        fprintf(stderr, "Error: Launcher does not have CAP_SYS_RAWIO.\n");
        fprintf(stderr, "Please run: sudo setcap cap_sys_rawio+eip %s\n", argv[0]);
        cap_free(caps);
        return 1;
    }

    // Set the capability as inheritable
    cap_value_t cap_list[] = { CAP_SYS_RAWIO };
    if (cap_set_flag(caps, CAP_INHERITABLE, 1, cap_list, CAP_SET) != 0) {
        perror("cap_set_flag inheritable");
        cap_free(caps);
        return 1;
    }

    if (cap_set_proc(caps) != 0) {
        perror("cap_set_proc");
        cap_free(caps);
        return 1;
    }

    cap_free(caps);

    // Allow capabilities to be inherited across execve
    if (prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_RAISE, CAP_SYS_RAWIO, 0, 0) != 0) {
        perror("prctl PR_CAP_AMBIENT_RAISE");
        return 1;
    }

    // Execute the target program
    execvp(argv[1], &argv[1]);

    // If we get here, exec failed
    fprintf(stderr, "Failed to execute %s: %s\n", argv[1], strerror(errno));
    return 1;
}
