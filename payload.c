#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <stdio.h>

#define CPUID_EXT_HYPERVISOR ((unsigned int)(1 << 31))

#define LOG_PORT 42

static void outb(uint16_t port, uint8_t value) {
	asm("outb %0,%1" : /* empty */ : "a" (value), "Nd" (port) : "memory");
}

static void out_string(uint16_t port, char* value) {
	for (char* p = value; *p; ++p)
		outb(port, *p);
}

static void out_string_max(uint16_t port, char* value, uint32_t max_len) {
	for (char* p = value; *p && (p - value) < max_len; ++p)
		outb(port, *p);
}

static void get_cpuid(unsigned leaf, unsigned* regs) {
    asm volatile("cpuid": "=a" (regs[0]), "=b" (regs[1]),
                  "=c" (regs[2]), "=d" (regs[3]) : "a" (leaf));
}

static void cpu_set_msr(uint32_t msr, uint32_t lo, uint32_t hi) {
   asm volatile("wrmsr" : : "a"(lo), "d"(hi), "c"(msr));
}

static void cpu_get_msr(uint32_t msr, uint32_t *lo, uint32_t *hi) {
   asm volatile("rdmsr" : "=a"(*lo), "=d"(*hi) : "c"(msr));
}

static void halt(uint32_t value) {
	asm("hlt" : /* empty */ : "a" (value) : "memory");
}

void
__attribute__((section(".start")))
_start(void) {
	out_string(LOG_PORT, "Greetings from the guest!\n");

	unsigned regs[] = {0, 0, 0, 0};
	get_cpuid(1, regs);
	if (regs[2] == CPUID_EXT_HYPERVISOR) {
		out_string(LOG_PORT, "We haz hypervizor!\n");
	}

	memset(regs, 0, sizeof(regs));
	get_cpuid(0x40000000, regs);

	char id[13] = {0};
	memcpy(id, &regs[1], 12);
	id[12] = 0;

	out_string(LOG_PORT, "Hypervisor ID: ");
	out_string_max(LOG_PORT, id, sizeof(id));
	out_string(LOG_PORT, "\n");

	cpu_set_msr(1, 2, 3);

	uint32_t lo, hi = 0;
	cpu_get_msr(1, &lo, &hi);

	halt(0);
}