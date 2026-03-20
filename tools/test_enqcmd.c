#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <stdint.h>
#include <immintrin.h>
#include <errno.h>
#include <sys/ioctl.h>

struct dsa_hw_desc {
    uint32_t pasid_priv;
    uint32_t flags_opcode;
    uint64_t completion_addr;
    uint64_t src_addr;
    uint64_t dst_addr;
    uint32_t xfer_size;
    uint16_t int_handle;
    uint16_t rsvd1;
    uint8_t  op_specific[24];
} __attribute__((aligned(64)));

struct dsa_completion {
    uint8_t  status;
    uint8_t  result;
    uint16_t rsvd;
    uint32_t bytes_completed;
    uint64_t fault_addr;
    uint8_t  op_specific[16];
} __attribute__((aligned(32)));

static inline int try_enqcmd(void *portal, const void *desc) {
    uint8_t retry;
    asm volatile(
        "enqcmd (%[src]), %[dst]\n\t"
        "setz %[retry]"
        : [retry] "=r" (retry)
        : [dst] "r" (portal), [src] "r" (desc)
        : "memory", "cc"
    );
    return retry;  // 1 = accepted, 0 = rejected
}

int main(int argc, char **argv) {
    const char *dev = argc > 1 ? argv[1] : "/dev/dsa/wq0.1";

    printf("=== ENQCMD Shared WQ Test ===\n");
    printf("Device: %s\n", dev);
    printf("PID: %d\n", getpid());

    int fd = open(dev, O_RDWR);
    if (fd < 0) {
        perror("open");
        printf("errno=%d\n", errno);
        return 1;
    }
    printf("Opened %s (fd=%d)\n", dev, fd);

    void *portal = mmap(NULL, 4096, PROT_WRITE, MAP_SHARED | MAP_POPULATE, fd, 0);
    if (portal == MAP_FAILED) {
        perror("mmap");
        printf("errno=%d\n", errno);
        close(fd);
        return 1;
    }
    printf("Portal mapped at %p\n", portal);

    struct dsa_hw_desc desc __attribute__((aligned(64)));
    struct dsa_completion comp __attribute__((aligned(32)));

    memset(&desc, 0, sizeof(desc));
    memset(&comp, 0, sizeof(comp));

    // NOOP with RCR + CRAV flags (bits 2,3)
    desc.flags_opcode = 0x0C;  // RCR|CRAV | opcode=0 (NOOP)
    desc.completion_addr = (uint64_t)&comp;

    printf("\nDescriptor dump:\n");
    printf("  pasid_priv:      0x%08x\n", desc.pasid_priv);
    printf("  flags_opcode:    0x%08x\n", desc.flags_opcode);
    printf("  completion_addr: 0x%016lx\n", (unsigned long)desc.completion_addr);
    printf("  desc address:    %p (aligned to %lu)\n", &desc, ((uintptr_t)&desc) % 64);

    printf("\nSubmitting NOOP via ENQCMD (max 100 retries)...\n");

    int accepted = 0;
    for (int i = 0; i < 100; i++) {
        int ok = try_enqcmd(portal, &desc);
        if (ok) {
            printf("ENQCMD accepted on attempt %d\n", i);
            accepted = 1;
            break;
        }
    }

    if (!accepted) {
        printf("ENQCMD rejected 100/100 times!\n");
        printf("\nPossible causes:\n");
        printf("  1. PASID not allocated (IA32_PASID MSR not set by kernel)\n");
        printf("  2. iommu_sva_bind_device() failed in idxd cdev open\n");
        printf("  3. WQ not properly configured for shared mode\n");
        printf("\nCheck: sudo rdmsr 0x0D93  (IA32_PASID MSR)\n");
        printf("Check: sudo dmesg | grep -i 'pasid\\|sva\\|idxd'\n");
    } else {
        printf("Polling completion...\n");
        volatile uint8_t *status = &comp.status;
        int spins = 0;
        while (*status == 0 && spins < 10000000) {
            _mm_pause();
            spins++;
        }
        printf("Completion status: 0x%02x after %d spins\n", *status, spins);
        if (*status == 0)
            printf("  -> Timed out waiting for completion\n");
        else if (*status == 1)
            printf("  -> SUCCESS\n");
        else
            printf("  -> Error status\n");
    }

    munmap(portal, 4096);
    close(fd);
    return 0;
}
