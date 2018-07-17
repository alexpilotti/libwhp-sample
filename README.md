# libwhp-sample
A full Windows Hypervisor Platform API Rust sample using
[libwhp](https://github.com/insula-rs/libwhp).

## Prerequisites

Make sure to have at least: 

* Windows 10 build 17134 (or above)
* Windows Server 1803 (or above)

Enable the Windows Hypervisor Platform and reboot:

```
Dism /Online /Enable-Feature /FeatureName:HypervisorPlatform
shutdown /r /t 0
```

The payload needs to be compiled using GCC e.g. using WSL
([Windows Subsystem for Linux](https://docs.microsoft.com/en-us/windows/wsl/install-win10)).
All we need is make, gcc and ld. For example on Ubuntu:

```
wsl sudo apt-get update
wsl sudo apt-get dist-upgrade -y
wsl sudo apt-get install gcc make binutils -y
```

Last but not least, install [Rust on Windows](https://www.rust-lang.org/en-US/install.html).

## Build and run

Build the payload:

```
wsl make
```

Now just build and run the sample:

```
cargo run
```

## What does the sample do?

* Checks for the hypervisor presence
* Creates a partition
* Sets various partition properties, like the allowed exit types and CPUID results
* Allocates and maps memory
* Creates a vCPU
* Sets up registers for long mode (64 bit)
* Reads the payload in memory (payload.img)
* Sets up the MMIO / IO port intruction emulator and related callbacks
* Starts the vCPU loop
* Handles various type of exits: CPUID, MSR read / write, IO port, Halt, etc
