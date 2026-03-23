/*
 * DSA Launcher - launches binaries with CAP_SYS_RAWIO capability.
 *
 * Build: gcc -O2 -o dsa_launcher dsa_launcher.c
 * Setup: setcap cap_sys_rawio+eip dsa_launcher
 */

#define _GNU_SOURCE
#include <errno.h>
#include <linux/capability.h>
#include <stdio.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <unistd.h>

static int enable_inheritable_rawio(void) {
    struct __user_cap_header_struct header = {
        .version = _LINUX_CAPABILITY_VERSION_3,
        .pid = 0,
    };
    struct __user_cap_data_struct data[_LINUX_CAPABILITY_U32S_3] = {{0}};
    int idx = CAP_TO_INDEX(CAP_SYS_RAWIO);
    __u32 mask = CAP_TO_MASK(CAP_SYS_RAWIO);

    if (syscall(SYS_capget, &header, data) != 0) {
        perror("capget");
        return -1;
    }

    if ((data[idx].permitted & mask) == 0) {
        fprintf(stderr, "CAP_SYS_RAWIO is not in the permitted set\n");
        return -1;
    }

    data[idx].inheritable |= mask;

    if (syscall(SYS_capset, &header, data) != 0) {
        perror("capset");
        return -1;
    }
    return 0;
}

int main(int argc, char *argv[]) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <program> [args...]\n", argv[0]);
        return 1;
    }

    if (enable_inheritable_rawio() != 0) {
        fprintf(stderr,
                "Hint: launcher must have file capability cap_sys_rawio+eip\n");
        return 1;
    }

    if (prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_RAISE, CAP_SYS_RAWIO, 0, 0) != 0) {
        perror("prctl PR_CAP_AMBIENT_RAISE");
        fprintf(stderr,
                "Hint: launcher must have file capability cap_sys_rawio+eip\n");
        return 1;
    }

    execvp(argv[1], &argv[1]);
    fprintf(stderr, "Failed to execute %s: %s\n", argv[1], strerror(errno));
    return 1;
}
