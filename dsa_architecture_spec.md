# intel

# INTEL® DATA STREAMING ACCELERATOR ARCHITECTURE SPECIFICATION

Order Number: 341204-006US

Revision: 3.0

June 2025

# Notices & Disclaimers

This document contains information on products in the design phase of development. The information here is subject to change without notice. Do not finalize a design with this information.

Intel technologies may require enabled hardware, software or service activation.

No product or component can be absolutely secure.

Your costs and results may vary.

You may not use or facilitate the use of this document in connection with any infringement or other legal analysis concerning Intel products described herein. You agree to grant Intel a non-exclusive, royalty-free license to any patent claim thereafter drafted which includes subject matter disclosed herein.

All product plans and roadmaps are subject to change without notice.

The products described may contain design defects or errors known as errata which may cause the product to deviate from published specifications. Current characterized errata are available on request.

Intel disclaims all express and implied warranties, including without limitation, the implied warranties of merchantability, fitness for a particular purpose, and non-infringement, as well as any warranty arising from course of performance, course of dealing, or usage in trade.

No license (express or implied, by estoppel or otherwise) to any intellectual property rights is granted by this document, with the sole exception that you may publish an unmodified copy. You may create software implementations based on this document and in compliance with the foregoing that are intended to execute on the Intel product(s) referenced in this document. No rights are granted to create modifications or derivatives of this document.

Copies of documents which have an order number and are referenced in this document, or other Intel literature, may be obtained by calling 1-800-548-4725, or go to: http://www.intel.com/design/literature.htm.

© Intel Corporation. Intel, the Intel logo, and other Intel marks are trademarks of Intel Corporation or its subsidiaries. Other names and brands may be claimed as the property of others.

# Table of Contents

# 1 Introduction 17

1.1 Audience 17

1.2 References 17

# 2 Overview 19

2.1 High Level Usages 19

2.2 Intel® DSA Features 20

2.2.1 Infrastructure Features 20

2.2.2 Data Operations 21

2.2.3 Control Operations 22

# 3 Intel Data Streaming Accelerator Architecture 23

3.1 Register and Software Programming Interface 23

3.2 Descriptors 24

3.3 Work Queues 24

3.3.1 Shared Work Queue (SWQ) 25

3.3.2 Dedicated Work Queue (DWQ) 26

3.4 Engines and Groups 26

3.5 Descriptor Processing 28

3.6 Descriptor Completion 28

3.7 Interrupts 29

3.8 Batch Descriptor Processing 31

3.9 Ordering and Fencing 32

3.10 Cache Control 34

3.11 Persistent Memory Support 34

3.12 Drain Descriptor 34

3.13 Address Translation 35

3.13.1 Work Queue Address Translation Controls 36

3.13.2 Page Fault Handling with PRS Enabled 36

3.13.3 Page Fault Handling with PRS Disabled 37

3.14 Inter-Domain Operations 37

3.14.1 Inter-Domain Permissions Table (IDPT) 39

3.14.2 Submitter Bitmap 40

3.14.3 Memory Window. 41

3.14.4 Inter-Domain Completions 42

3.14.5 Memory Window Modification 42

3.15 Administrative Commands 42

3.16 Virtualization 45

# 4 Quality of Service Control. 47

4.1 Work Dispatch Priority 47

4.2 Traffic Classes 47

4.3 Read Buffer Allocation 48

4.4 Latency Control 48

4.5 Low Bandwidth Memory 49

4.6 Bandwidth Control 50

# 5 Error Handling 51

5.1 Device Enable Checks 51

5.2 WQ Enable Checks 53

5.3 Descriptor Submission Checks 54

5.4 Descriptor Checks 55

5.5 Descriptor Reserved Field Checking 58

5.6 Inter-Domain Permissions Table Entry Checks 61

5.7 Device Halt State 62

5.8 Error Codes 63

5.8.1 Operation Status Codes 63

5.8.2 Other Software Error Codes 66

5.8.3 Administrative Command Error Codes 66

5.9 Event Log 69

5.9.1 Event Log Entry 70

# 6 Performance Monitoring. 75

6.1 Perfmon Discovery and Enumeration 75

6.2 Perfmon Configuration Registers 76

6.3 Event Counters 77

6.3.1 Counter Overflow 77

6.3.2 Counter Stop and Resume 78

6.4 Filter Support 78

6.5 Event Programming Considerations 78

6.6 Interrupt Generation 80

# 7 Reference Software Architecture 81

7.1 Kernel Mode Driver 81

7.2 User Mode Driver 81

7.3 Software Requirements for Handling Non-Blocking Page Faults 82

7.4 Software Requirements for Inter-Domain Operations 83

7.5 Virtualization Software 84

7.5.1 Virtual Intel® DSA Device 84

7.5.2 Portal Virtualization 86

7.5.3 SVM and PASID Virtualization 86

7.5.4 Interrupt Virtualization 87

7.5.5 Capability Virtualization 89

7.5.6 Virtualization of Inter-Domain Features 89

7.5.7 State Migration During VM Migration 90

7.5.8 Virtualization of Event Log 90

# 8 Descriptor Formats 93

8.1 Common Descriptor Fields 93

8.1.1 Trusted Fields 93

8.1.2 Operation 94

8.1.3 Flags 95

8.1.4 Completion Record Address 100

8.1.5 Source Address 100

8.1.6 Destination Address 100

8.1.7 Transfer Size 100

8.1.8 Completion Interrupt Handle 101

8.1.9 Element Count 101

8.1.10 Data Types 101

8.1.11 Compute Type 103

8.1.12 Compute Flags 103

8.1.13 Inter-Domain Selector 105

8.1.14 Scatter Gather List (SGL) Format 105

8.2 Completion Record 107

8.2.1 Status 107

8.2.2 Result 107

8.2.3 Fault Info 109

8.2.4 Bytes Completed 109

8.2.5 Fault Address 109

8.2.6 Invalid Flags 110

8.3 Descriptor Types 111

8.3.1 No-op 111

8.3.2 Batch 112

8.3.3 Drain 114

8.3.4 Memory Move 116

8.3.5 Fill 117

8.3.6 Compare 118

8.3.7 Compare Pattern 119

8.3.8 Create Delta Record 120

8.3.9 Apply Delta Record 122

8.3.10 Memory Copy with Dualcast 124

8.3.11 Translation Fetch 125

8.3.12 CRC Generation 127

8.3.13 Copy with CRC Generation 129

8.3.14 DIF Check 130

8.3.15 DIF Insert 131

8.3.16 DIF Strip 132

8.3.17 DIF Update 133

8.3.18 DIX Generate 137

8.3.19 Type Conversion 138

8.3.20 Reduce 140

8.3.21 Reduce with Dualcast 142

8.3.22 Gather Reduce 144

8.3.23 Gather Copy 146

8.3.24 Scatter Copy 148

8.3.25 Scatter Fill 150

8.3.26 Cache Flush 152

8.3.27 Update Window 153

8.3.28 Inter-Domain Copy 155

8.3.29 Inter-Domain Fill 156

8.3.30 Inter-Domain Compare 157

8.3.31 Inter-Domain Compare Pattern 158

# 9 Register Descriptions 159

9.1 PCI Configuration Space Registers 160

9.1.1 Base Address Registers (BAR) 160

9.1.2 MSI-X Capability 160

9.1.3 Address Translation Capabilities 160

9.1.4 VC Capability 162

9.2 Configuration and Control Registers (BAR0) 163

9.2.1 Version Register (VERSION) 167

9.2.2 General Capabilities Register (GENCAP) 168

9.2.3 WQ Capabilities Register (WQCAP) 171

9.2.4 Group Capabilities Register (GRPCAP) 173

9.2.5 Engine Capabilities Register (ENGCAP) 174

9.2.6 Operations Capabilities Register (OPCAP) 175

9.2.7 Table Offsets Register (OFFSETS) 176

9.2.8 General Configuration Register (GENCFG) 177

9.2.9 General Control Register (GENCTRL) 178

9.2.10 General Status Register (GENSTS) 179

9.2.11 Interrupt Cause Register (INTCAUSE) 180

9.2.12 Command Register (CMD) 181

9.2.13 Command Status Register (CMDSTATUS) 184

9.2.14 Command Capabilities Register (CMDCAP) 185

9.2.15 Software Error Register (SWERROR) 186

9.2.16 Event Log Configuration Register (EVLCFG) 189

9.2.17 Event Log Status Register (EVLSTATUS) 190

9.2.18 Inter-Domain Capabilities Register (IDCAP) 191

9.2.19 Inter-Domain Bitmap Register (IDBR) 192

9.2.20 Command Parameter Register (CMDPARAM) 193

9.2.21 Dummy Portal (DUMMY) 194

9.2.22 MSI-X Permissions Table (MSIXPERM) 195

9.2.23 Group Configuration Table (GRPCFG) 196

9.2.24 WQ Configuration Table (WQCFG) 200

9.2.25 Performance Monitoring Registers 208

9.2.26 MSI-X Table 217

9.2.27 MSI-X Pending Bit Array 217

9.2.28 Interrupt Message Storage 218

9.2.29 Inter-Domain Permissions Table (IDPT) 219

9.2.30 DSA Capabilities (DSACAPO) 222

9.2.31 DSA Capabilities (DSACAP1) 223

9.2.32 DSA Capabilities (DSACAP2) 224

9.3 Portals (BAR2) 225

# Appendix A CRC Computation 227

# Appendix B Data Integrity Field (DIF) 229

B.1 DIF Check 231

B.2 DIF Insert 231

B.3 DIF Strip 231

B.4 DIF Update 231

B.5 DIX Generate 232

# Appendix C PCIe Configuration Registers 233

# Appendix D Performance Monitoring Events 269

D.1 Architectural Performance Monitoring Events 269

D.1.1 Version 1. 269

D.1.2 Version 2 273

D.2 Model-Specific Performance Monitoring Events 275

D.2.1 Version 1. 276

D.2.2 Version 2 278

D.3 Event Configuration Examples 281

Appendix E Floating Point Operations 285

E.1 Floating Point Data Types 285

E.2 Compatibility with CPU 286

Appendix F Summary of Key Architecture Extensions 287

# List of Figures

Figure 3-1: Abstracted Internal Block Diagram of Intel® DSA. 24

Figure 3-2: Sample Group Configuration 1 27

Figure 3-3: Sample Group Configuration 2 28

Figure 3-4: Inter-Domain Permissions Table Entry Types. 39

Figure 5-1: Event Log Entry 70

Figure 7-1: Example Software Flow for an Inter-Domain Operation 84

Figure 7-2: Intel® Scalable IOV for Intel® DSA 85

Figure 7-3: Guest Steps to Handle Interrupt Handle Revocation 88

Figure 8-1: Delta Record Usage 123

Figure 8-2: Illustration of Gather Reduce operation. 144

Figure 9-1: MMIO Register Map. 163

Figure 9-2: Portals 226

Figure 9-3: Floating Point Data Types. 285

# List of Tables

Table 1-1: References 17

Table 2-1: Intel® DSA Data Operations. 22

Table 2-2: Intel® DSA Control Operations. 22

Table 3-1: Interrupt Delivery. 30

Table 5-1: Handling of Software Errors. 52

Table 5-2: Completion Interrupt Handle Checks. 55

Table 5-3: Supported Flags and Reserved Fields by Operations. 60

Table 5-4: Conditional Reserved Field Checking 60

Table 5-5: Operation Types with Required (Must be 1) Flags. 61

Table 5-6: Operation Status Codes 66

Table 5-7: Other Software Error Codes 66

Table 5-8: Administrative Command Error Codes 68

Table 5-9: Event Log Entry Format. 73

Table 6-1: Event Categories. 76

Table 6-2: Filter Types and Mask .79

Table 8-1: Descriptor Trusted Fields 93

Table 8-2: Operation Types 94

Table 8-3: Descriptor Flags 98

Table 8-4: Cache Control Flags. 99

Table 8-5: Data Types. 102

Table 8-6: Compute Operations 103

Table 8-7: Compute Flags. 104

Table 8-8: Inter-Domain Selector. 105

Table 8-9: SGL Format. 106

Table 8-10: Completion Record Status field. 107

Table 8-11: Numeric Exception Result Flags .108

Table 8-12: Completion Status for Compute Operations. 108

Table 8-13: Completion Record Fault Info .109

Table 8-14: Batch Operation-Specific Flags 113

Table 8-15: Drain Operation-Specific Flags 115

Table 8-16 : Fill Operation-Specific Flags............117

Table 8-17: Completion Status for Compare Descriptor. 118

Table 8-18 : Translation Fetch Operation-Specific Flags 126

Table 8-19: CRC Generation Operation-Specific Flags. 128

Table 8-20: Type Conversion Operation-Specific Flags. 139

Table 8-21: Reduce Operation-Specific Flags 141

Table 8-22: Reduce with Dualcast Operation-Specific Flags. 143

Table 8-23: Gather Copy Operation-Specific Flags 147

Table 8-24: Scatter Copy Operation-Specific Flags. 149

Table 8-25: Scatter Fill Operation-Specific Flags. 151

Table 8-26: Update Window - Window Flags. 154

Table 8-27: Update Window Operation-Specific Flags 154

Table 8-28: Inter-Domain Copy Operation-Specific Flags. 155

Table 8-29: Inter-Domain Fill Operation-Specific Flags 156

Table 8-30: Inter-Domain Compare Operation-Specific Flags 157

Table 8-31: Inter-Domain Compare Pattern Operation-Specific Flags. 158

Table 9-1: Register Attributes. 160

Table 9-2: Address Translation Modes. 161

Table 9-3: MMIO Register Initial Values. 166

Table 9-4: Read-Only MMIO Registers 166

Table 9-5: Administrative Commands. 183

Table 9-6: Default Commands Supported. 185

Table 9-7: Work Queue Configuration Support 200

Table 9-8: Perfmon Register Read-Only Status 208

Table 9-9: Filter Configuration Register Offsets 215

Table 9-10: Inter-Domain Permissions Table Entry Read-Only Status............219

Table 9-11: Supported Portal Operations 225

Table 9-12: Numeric Range for Floating Point Types 285

Table 9-13: NaN and Infinity for Floating Point Types. 286


Revision History


<table><tr><td>Date</td><td>Revision</td><td>Description</td></tr><tr><td>November 2019</td><td>Rev 1.0</td><td>- Initial release of the document.</td></tr><tr><td>October 2020</td><td>Rev 1.1</td><td>- Addressed errata and omissions in Rev 1.0.- Added guarantee of descriptor ordering under certain conditions.- Added Command Capabilities register (CMDCAP).- Added Dummy Portal.- Added WQ ATS Disable.- Added constraint on the value of Global Bandwidth Token Limit.- Added Release Interrupt Handle command.- Added information on Performance Monitoring.- Added details on Create Delta Record and Apply Delta Record.- Added details on CRC and DIF operations.- Removed Interrupt Handle Request capability. Instead, the Command Capabilities register is used to indicate support for the Request Interrupt Handle command.- Clarified use of the Request Interrupt Handle command and described interrupt handle revocation.- Changed description of Abort All command to require that no descriptors be submitted to the device while it is being processed.- Changed Command register to write-only.- Clarified intended use of unlimited portals for SWQs.- Clarified behavior of IMS portals when IMS is not available.- Clarified behavior of Ignore field in MSI-X Permissions and IMS.</td></tr><tr><td>October 2021</td><td>Rev 1.2</td><td>- Addressed errata and omissions in Rev 1.1.- Added Interrupt Handles Revoked in INTCAUSE.- Changed the process of interrupt handle revocation and added pseudocode describing the software sequence to support it.- Changed the operand type for the Release Interrupt Handle command.- Renamed Bandwidth Tokens to Read Buffers, including renaming the associated fields in GRPCAP, GENCFG, and GRPCFG. (Note, there are not change bars for this name change.)- Clarified the behavior and usage of Read Buffer controls.- Moved the table of Administrative Command Error Codes from section 9.2.13 to section 5.7.3.- Clarified that the Strict Ordering flag in a descriptor guarantees ordering of memory writes both within the device and without.- Clarified that software must not rely on the values of fields in completion records for error codes where the fields do not have specified meanings.- Clarified that bits 11:0 of the Fault Address field in a completion record or the SWERROR register may be reported as 0.</td></tr><tr><td></td><td></td><td>- Clarified that the Cache Control flag in descriptors is reserved if the corresponding Cache Control Support field in GENCAP is 0.</td></tr><tr><td>September 2022</td><td>Rev 2.0</td><td>- Added Inter-domain operations and related registers including Inter-Domain Capabilities Register, Permissions Table, Submitter bitmap, etc. 
- Extensions to Fill, CRC, and Data Integrity Field (DIF) operations. 
- Support for Event Log. 
- Support for per WQ OPCFG. 
- Support for Translation Fetch descriptor. 
- Support for engine pipeline depth control. 
- Error code changes, and updates to SWERROR register and Completion Record Fault Info. 
- Support for WQ PRS Disable control. 
- Removed Steering Tag Support. 
- Removed support for Cache Flush with write-back semantics. 
- Perfmon updates. 
- Corrected typographical errors.</td></tr><tr><td>October 2024</td><td>Rev 2.1</td><td>- Removed Inter Domain Cache Flush. 
- Clarifications and errata.</td></tr><tr><td>June 2025</td><td>Rev 3.0</td><td>- Added Scatter-Gather operations. 
- Added Type Conversion and Reduce operations. 
- Added Read and Write Bandwidth Limit fields in GRPCFG. 
- Support single-entry Batch. 
- Clarified and extended specification of cache control flags. 
- Added capability registers DSACAP0, DSACAP1, and DSACAP2.</td></tr></table>


Glossary


<table><tr><td>Acronym</td><td>Term</td><td>Description</td></tr><tr><td>ALU</td><td>Arithmetic and Logic Unit</td><td>Hardware logic to perform basic math computation with different types of data.</td></tr><tr><td>ATS</td><td>Address Translation Services</td><td>A protocol defined by the PCI Express specification to support address translations by a device and to issue ATC invalidations.</td></tr><tr><td>ATC</td><td>Address Translation Cache</td><td>A structure in the device that stores translated addresses. Also known as Device TLB.</td></tr><tr><td>BD</td><td>Batch descriptor</td><td>A descriptor that refers to an array of descriptors in memory, to allow submitting multiple work descriptors at once.</td></tr><tr><td></td><td>Completion Record</td><td>A 32-byte data structure in memory that is written by the device when an operation completes.</td></tr><tr><td></td><td>Dedicated Mode</td><td>A mode that allows a single software client to submit work without unnecessary overhead.</td></tr><tr><td></td><td>Descriptor</td><td>A 64-byte data structure written to the device to specify work to be performed.</td></tr><tr><td>DWQ</td><td>Dedicated Work Queue</td><td>A work queue used by a single software client to submit work.</td></tr><tr><td>DMWr</td><td>Deferrable Memory Write</td><td>A type of PCI Express transaction that allows the device to defer (temporarily refuse) the write request.</td></tr><tr><td></td><td>Engine</td><td>An independent operational unit within the Intel DSA device.</td></tr><tr><td>ENQCMD</td><td>Enqueue Command</td><td>An Intel® 64 CPU instruction to enqueue a command to a shared work queue using Deferrable Memory Write (DMWr).</td></tr><tr><td>ENQCMDS</td><td>Enqueue Command Supervisor</td><td>An Intel® 64 CPU instruction to enqueue a command with Supervisor permissions (from privileged software) to a shared work queue using Deferrable Memory Write (DMWr).</td></tr><tr><td>FP</td><td>Floating Point</td><td>A data representation to support non-integer numbers.</td></tr><tr><td>IDPT</td><td>Inter-Domain Permissions Table</td><td>Table to manage inter-domain operations.</td></tr><tr><td>IDPTE</td><td>Inter-Domain Permissions Table Entry</td><td>Any entry in the IDPT.</td></tr><tr><td>IMS</td><td>Interrupt Message Storage</td><td>A Scalable I/O Virtualization feature used to store MSI messages in a device-specific manner.</td></tr><tr><td>IOMMU</td><td>I/O Memory Management Unit</td><td>DMA Remapping Hardware Unit as defined by Intel® Virtualization Technology for Directed I/O.</td></tr><tr><td></td><td>Group</td><td>A configurable set of work queues and engines.</td></tr><tr><td>MMIO</td><td>Memory-Mapped I/O</td><td>Access to an I/O device via the processor's physical-memory address space using ordinary processor memory-access instructions.</td></tr><tr><td>MOVDIR64B</td><td>Move 64-Bytes as Direct Store</td><td>An Intel® 64 CPU instruction used to enqueue a command to a dedicated work queue using a 64-byte memory write.</td></tr><tr><td>MSI</td><td>Message Signaled Interrupt</td><td>A memory write operation to a pre-defined address to generate an interrupt.</td></tr><tr><td>MSI-X</td><td></td><td>A PCI Express feature used to configure Message Signaled Interrupts.</td></tr><tr><td>PASID</td><td>Process Address Space Identifier</td><td>A value used in memory transactions to convey the address space on the host of an address used by the device.</td></tr><tr><td>PM</td><td>Persistent Memory</td><td>Memory that retains state when power is removed, such as battery-backed DRAM.</td></tr><tr><td>PRS</td><td>Page Request Service</td><td>A protocol defined by the PCI Express specification for a device to report recoverable page-faults and receive page-fault responses.</td></tr><tr><td>RSVD</td><td>Reserved</td><td>Any field that is described as reserved in this specification must be written as 0 by software. Generally, hardware reports an error if a reserved field is non-zero, but it may not do so in all cases. If software sets a reserved field to a non-zero value and no error is reported, behavior is undefined.</td></tr><tr><td>SGL</td><td>Scatter-Gather List</td><td>A list of memory locations to read or write.</td></tr><tr><td>SoC</td><td>System-on-chip</td><td>An integrated chip composed of host processors, accelerators, memory, and I/O agents.</td></tr><tr><td>SR-IOV</td><td>Single Root I/O Virtualization</td><td>A PCI Express standard for virtualizing PCI Express endpoint device interfaces.</td></tr><tr><td>SVM</td><td>Shared Virtual Memory</td><td>Ability for an accelerator or I/O device to operate in the same virtual memory space as applications on host processors. It also implies ability to operate from page-able memory, avoiding functional requirements to pin memory for DMA operations.</td></tr><tr><td></td><td>Shared Mode</td><td>A mode that allows multiple software clients to concurrently submit work.</td></tr><tr><td>SWQ</td><td>Shared Work Queue</td><td>A work queue that allows multiple software clients to concurrently submit work.</td></tr><tr><td>TC</td><td>Traffic Class</td><td>A PCI Express feature that allows differentiation of transactions to apply appropriate servicing policies.</td></tr><tr><td>VDCM</td><td>Virtual Device Composition Module</td><td>A software component that is part of a VMM, which composes a virtual device and makes it available to a VM.</td></tr><tr><td>VDEV</td><td>Virtual Device</td><td>A virtual device implemented by VDCM.</td></tr><tr><td>WD</td><td>Work Descriptor</td><td>A descriptor that specifies a DMA operation.</td></tr><tr><td>WQ</td><td>Work Queue</td><td>A queue in the device used to store descriptors submitted by software until they can be dispatched.</td></tr></table>

S

# 1 Introduction

This document describes the architecture of the Intel® Data Streaming Accelerator (Intel® DSA), including the extensions in the  $2^{\text{nd}}$  and  $3^{\text{rd}}$  generations of Intel DSA. Intel DSA is a high-performance data copy and transformation accelerator integrated in Intel® processors, targeted for optimizing streaming data movement, transformation, and scatter-gather operations common with applications for high-performance storage, networking, persistent memory, AI, and other data processing applications.

# 1.1 Audience

The intended audience for this specification is hardware engineers and SoC architects building compliant hardware implementations, device driver software developers programming the device, virtualization software providers efficiently enabling sharing and virtualization of the device, and application or library developers utilizing Intel DSA operations.

# 1.2 References

<table><tr><td>Description</td></tr><tr><td>Intel® 64 and IA-32 Architectures Software Developer&#x27;s Manuals
https://software.intel.com/content/www/us/en/develop/articles/intel-sdm.html</td></tr><tr><td>PCI Express Base Specification
http://www.pcisig.com/specifications/pciexpress</td></tr><tr><td>Intel® Virtualization Technology for Directed I/O Specification
https://software.intel.com/content/www/us/en/develop/download/intel-virtualization-technology-for-directed-io-architecture-specification.html</td></tr><tr><td>Intel® Scalable I/O Virtualization Technical Specification
https://software.intel.com/content/www/us/en/develop/download/intel-scalable-io-virtualization-technical-specification.html</td></tr><tr><td>RFC 3720, Internet Small Computer Systems Interface
http://www.ietf.org/rfc/rfc3720.txt</td></tr><tr><td>NVM Express NVM Command Set Specification
https://nvmexpress.org/specifications</td></tr></table>

Table 1-1: References

# 2 Overview

The goal of Intel DSA is to provide higher overall system performance for data mover and transformation operations, while freeing up CPU cycles for higher level functions. Intel DSA hardware supports high-performance data mover capability to/from volatile memory, persistent memory, memory-mapped I/O, and through a Non-Transparent Bridge (NTB) in the SoC to/from remote volatile and persistent memory on another node in a cluster. It provides a PCI Express compatible programming interface to the Operating System and can be controlled through a device driver.

In addition to performing basic data mover operations, Intel DSA is designed to perform some higher-level transformation operations on memory. For example, it can generate and test CRC checksum or Data Integrity Field (DIF) on the memory region, to support usages typical with storage and networking applications. It can compare memory for equality, generate a delta record, and apply a delta record to a buffer. The compare and delta generate/apply functions may be utilized by applications such as VM migration, VM fast check-pointing, and software managed memory dedduplication usages. Reduction and scatter-gather operations are designed for use in AI and HPC applications.

Intel DSA may also be used for data movement between different address spaces by using the Inter-Domain capabilities of the device. This may have application in networking, for example with a virtual switch implementation to efficiently copy data between virtual machines, or to speed up inter-process communication (IPC) primitives in the OS or VMM. It may also be used for message and data passing between processes in application domains like HPC and Machine Learning.

# 2.1 High Level Usages

This section summarizes some of the envisioned data movement and transformation usages for Intel DSA.

- Datacenter: As a data movement offload engine to reduce datacenter tax for memory copying, zeroing, etc., to free up CPU cycles from mundane infrastructure work.

- Storage: For data movement in storage appliances, both within the node and across nodes using Non-Transparent Bridge (NTB); and for CRC generation and Data Integrity Field (DIF) generation, with or without simultaneously moving data.

- Networking: For data copy in packet processing pipelines. An example usage is virtual switch offload for inter-VM packet switching.

Deduplication: For comparing memory pages for equality to support memory dedduplication.

- VM Migration and Fast Checkpointing: VM fast checkpointing and VM migration flows require the VMM to identify a VM's modified pages and send them efficiently to the destination machine, with minimal network traffic and latency. Intel DSA delta operations generate diffs of pages, enabling the VMM to send only the modified data to the destination, reducing network traffic.

- Data movement between peer devices: May be used for data movement between a peer accelerator device and host memory or between two peer devices to free up CPU cycles from such infrastructure work.

- Data movement to/from/between virtual machines: To free up CPU cores from performing routine infrastructure tasks including moving data between virtual machines, containers, and bare metal hosts.

- HPC and AI workloads: Reduction operations to accelerate intra-node all-reduce communication across MPI ranks. Scatter-gather operations to accelerate embedding operations for DLRM, HPC, and DB analytics applications.

# 2.2 Intel® DSA Features

Intel DSA features include 1) infrastructure features, which are basic features to help with programmability, performance, and efficiency; 2) data operations, which are the actual data DMA and other transformation operations; and 3) control operations. The following sections give an overview of these features.

# 2.2.1 Infrastructure Features

The following infrastructure features are supported by Intel DSA.

- Shared Virtual Memory (SVM): SVM allows user level applications to submit commands to the device directly, with virtual addresses in the descriptors. It supports translating virtual addresses to physical addresses using IOMMU including handling page faults. The virtual address ranges referenced by a descriptor may span multiple pages. Intel DSA also supports the use of physical addresses, as long as each data buffer specified in the descriptor is contiguous in physical memory.

- Partial descriptor completion: With SVM, an operation may encounter a page fault during address translation. Software can control whether the device is to continue processing after waiting for resolution of a page fault or terminate processing of a descriptor that encounters a page fault and proceed to the next descriptor. If processing of a descriptor is terminated, the completion record indicates to software the amount of work completed and information about the page fault so that software can resolve the fault and restart the operation from the point where it stopped.

- Block on fault: As an alternative to partial descriptor completion, when the device encounters a page fault it can coordinate with system software to resolve the fault and continue the operation transparently to the software that submitted the descriptor.

- Batch processing: A Batch descriptor points to an array of work descriptors (i.e., descriptors with actual data operations). When processing a batch descriptor, the device fetches the work descriptors from the specified virtual memory address and processes them.

- **Stateless device:** Descriptors are designed so that all information required for processing the descriptor comes in the descriptor itself. This allows the device to store little client specific state, which improves its scalability. The only exception is the completion interrupt message, when used, because it must be configured by trusted software.

- Cache allocation control: This allows applications to specify whether output data is allocated in the cache or is sent to memory without cache allocation. Completion records are always allocated in the cache.

- Shared Work Queue (SWQ) support: Shared Work Queues (SWQ) enable scalable work submission using Deferrable Memory Write transactions, which indicate whether the work was accepted into the WQ.

Dedicated Work Queue (DWQ) support: Dedicated Work Queues (DWQ) enable high-throughput work submission using 64-byte Memory Write transactions.

- QoS support: Intel DSA supports several features that allow the kernel driver to separately control access to device resources by different guests and applications.

- Intel® Scalable IOV support: Intel Scalable IO Virtualization improves scalability of device assignment, allowing a VMM to share the device across many more VMs than would be possible using SR-IOV.

- Persistent Memory features: Configuration registers and descriptor flags allow software to indicate writes to durable memory (such as battery-backed DRAM) and specify the durability and ordering semantics to the SoC.

- Inter-domain support: Intel DSA supports features that allow a single descriptor to perform certain data operations spanning different address space domains.

# 2.2.2 Data Operations

The following data operations are supported by Intel DSA. See chapter 8 for details on these operations

<table><tr><td>Operation</td><td>Type</td><td>Description</td></tr><tr><td rowspan="5">Move</td><td>Memory Move</td><td>Transfer data from a source address to destination address. 
Source and destination ranges can be either in main memory or MMIO.</td></tr><tr><td>CRC Generation</td><td>Generate CRC checksum on the transferred data.</td></tr><tr><td>DIF/DIX</td><td>Data Integrity Field (DIF) check. 
DIF insert, strip, or update while transferring data. 
Compute DIF for each block of source data and write to the destination address.</td></tr><tr><td>Dualcast</td><td>Copy data simultaneously to two destination locations.</td></tr><tr><td>Scatter-Gather</td><td>Copy data to or from a list of noncontiguous buffers.</td></tr><tr><td>Fill</td><td>Memory Fill</td><td>Fill memory range with a fixed pattern.</td></tr><tr><td rowspan="4">Compare</td><td>Memory Compare</td><td>Compare two source buffers and return whether the buffers are identical.</td></tr><tr><td>Create Delta Record</td><td>Create a delta record containing the differences between the original and modified buffers. The size of the delta record is bounded, and the device signals an overflow if the differences exceed the bound.</td></tr><tr><td>Apply Delta Record</td><td>Merge a delta record with the original source buffer to produce a copy of the modified buffer at the destination location.</td></tr><tr><td>Pattern/Zero Detect</td><td>Special case of compare where instead of the second input buffer, an 8-byte pattern is specified. Pattern may be zero.</td></tr><tr><td>Flush</td><td>Cache Flush</td><td>Evict all lines in a given address range from all levels of CPU caches.</td></tr><tr><td>Inter-Domain</td><td>Copy, Fill, Compare</td><td>Data operations across address domains initiated by privileged or unprivileged software.</td></tr><tr><td>Compute</td><td>Type Conversion Reduce</td><td>Element-wise ALU and data reduction operations.</td></tr></table>


Table 2-1: Intel® DSA Data Operations


# 2.2.3 Control Operations

The following control operations are supported by Intel DSA. Some of these commands are issued using descriptors and some are issued using the Command register. See sections 9.2.12 and 8.3 for details.

<table><tr><td>Operation</td><td>Type</td><td>Description</td></tr><tr><td rowspan="2">Enable / Disable / Reset</td><td>Device</td><td>Manage the device as a whole.</td></tr><tr><td>WQ</td><td>Manage individual WQs.</td></tr><tr><td>Drain</td><td>Current client</td><td>Drain all in-flight work requests from the current client.</td></tr><tr><td rowspan="3">Drain / Abort</td><td>Specified client</td><td>Drain or abort in-flight work requests from the specified client.</td></tr><tr><td>Work Queue</td><td>Drain or abort in-flight work requests in specified work queue.</td></tr><tr><td>All</td><td>Drain all in-flight work requests in the device.</td></tr><tr><td>No-op</td><td>Null operation</td><td>Performs no operation but can signal completion.</td></tr><tr><td>Update Window</td><td>Inter-domain</td><td>Control parameters of memory windows used by inter-domain operations.</td></tr></table>

Table 2-2: Intel® DSA Control Operations

# 3 Intel® Data Streaming Accelerator Architecture

This chapter describes the Intel DSA architecture in detail. Each SoC may support any number of Intel DSA device instances. A multi-socket server platform may support multiple such SoCs. From a software perspective, each instance is exposed as a single Root Complex Integrated Endpoint. Each instance is under the scope of a DMA Remapping hardware unit (also called an IOMMU). Each Intel DSA instance is behind a single DMA Remapping hardware unit, but depending on the SoC design, different device instances can be behind the same or different DMA Remapping hardware units.

Intel DSA supports an Address Translation Cache (ATC) and interacts with DMA Remapping hardware using the PCIe-defined Address Translation Services (ATS), Process Address Space ID (PASID), and Page Request Services (PRS) capabilities. The PASID TLP prefix is added to upstream requests to support both Shared Virtual Memory (SVM) and Intel Scalable I/O Virtualization (Intel Scalable IOV). The device utilizes the DMA Remapping hardware to translate DMA addresses to host physical addresses. Depending on the usage, a DMA address can be a Host Virtual Address (HVA), Guest Virtual Address (GVA), Guest Physical Address (GPA), or I/O Virtual Address (IOVA). Intel DSA supports additional PCI Express capabilities, including Advanced Error Reporting (AER) and MSI-X.

The Intel DSA architecture is designed to support Intel Scalable I/O Virtualization. The device can be shared directly with multiple VMs in a secure and isolated manner to achieve high throughput. Sections 3.16 and 7.3 describe the virtualization features in more detail.

Figure 3-1 illustrates the high-level blocks within the Intel DSA device at a conceptual level. Downstream work requests from clients are received on the I/O fabric interface. Upstream read, write, and address translation operations are sent on that interface. The device includes configuration registers, Work Queues (WQ) to hold descriptors submitted by software, arbiters used to implement QoS and fairness policies, processing engines, an address translation and caching interface, and a memory read/write interface. The batch processing unit processes Batch descriptors by reading the array of descriptors from memory. The work descriptor processing unit has stages to read memory, perform the requested operation on the data, generate output data, and write output data, completion records, and interrupt messages.

The WQ configuration allows software to configure each WQ either as a Shared Work Queue (SWQ) that can be shared by multiple software components, or as a Dedicated Work Queue (DWQ) that is assigned to a single software component at a time. The configuration also allows software to control which WQs feed into which engines and the relative priorities of the WQs feeding each engine.

# 3.1 Register and Software Programming Interface

Intel DSA is software compatible with the standard PCI Express configuration mechanism and implements a PCI header and extended space in its configuration-mapped register set.

Memory-mapped I/O registers provide status and control of device operation. Capability, configuration, and work submission registers (portals) are accessible through the MMIO regions defined by the BAR0 and BAR2 registers described in section 9.1.1. Each portal is on a separate 4K page so that they may be independently mapped into different address spaces (clients) using CPU page tables.

![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/c815b8f001fa2735c9377c53242ad6e9b0624432d454d2c40803ead9b81b01f8.jpg)



Figure 3-1: Abstracted Internal Block Diagram of Intel® DSA


# 3.2 Descriptors

Software specifies work for the device using descriptors. Descriptors specify the type of operation for the device to perform, addresses of data and status buffers, immediate operands, completion attributes, etc. See chapter 8 for descriptor formats and details. The completion attributes specify the address to write the completion record, and optionally, the information needed to generate a completion interrupt.

Intel DSA avoids maintaining client specific state on the device. All information to process a descriptor comes in the descriptor itself. This improves shareability of the device among user-mode applications, as well as among different virtual machines or machine containers in a virtualized system.

A descriptor may contain an operation and associated parameters (called a Work descriptor), or it can contain the address of an array of work descriptors (called a Batch descriptor). Software prepares the descriptor in memory and submits the descriptor to a Work Queue (WQ) of the device. The device dispatches descriptors from the work queues to the engines for processing. When an engine completes a descriptor or encounters certain faults or errors that result in an abort, it notifies the host software by either writing to a completion record in host memory, issuing an interrupt, or both.

# 3.3 Work Queues

Work queues (WQs) are on-device storage to contain descriptors that have been submitted to the device. The WQ Capability register indicates the number of work queues and the amount of work queue

storage available on the device. Software configures how many work queues are enabled and divides the available WQ space among the active WQs.

The WQ Configuration Table is used to configure the WQs. Prior to enabling the device, software configures the size of each WQ. Unused WQs have a size of 0. Other parameters of each WQ can be configured later, prior to enabling the WQ. In some configurations, the WQ size and other aspects of the WQ configuration are read-only. See section 9.2.24 for details on the WQ Configuration Table.

Each work queue can be configured to run in one of two modes, Dedicated or Shared. The WQ Capability register indicates support for Dedicated and Shared modes. Controls in the WQ Configuration Table allow software to configure the mode of each WQ. The mode of a WQ can only be changed while the WQ is Disabled. See the specifications for the WQ Capability register, the WQ Configuration Table, and the Command register in section 9.2 for details on configuring and enabling Work Queues.

Descriptors are submitted to work queues via special registers called portals. Each portal is in a separate 4 KB page in device MMIO space. There are four portals per WQ:

Unlimited MSI-X Portal

- Unlimited IMS Portal

- Limited MSI-X Portal

Limited IMS Portal

The address of the portal used to submit a descriptor allows the device to determine which WQ to place the descriptor in, whether the portal is limited or unlimited, and which interrupt table to use for the completion interrupt. See section 9.3 for details about portals.

See section 3.3.1, "Shared Work Queue," for the usage of limited and unlimited portals. For Dedicated WQs, there is no difference between the limited and unlimited portals.

See section 3.7, "Interrupts," for the usage of MSI-X and IMS portals. For a descriptor that does not request an interrupt, it doesn't matter whether it is submitted to an MSI-X portal or an IMS portal. The IMS portals do not exist if IMS is not supported, so a descriptor written to an address that would normally correspond to an IMS portal is discarded without reporting an error. If the descriptor was submitted with DMWr, a Retry response is returned.

# 3.3.1 Shared Work Queue (SWQ)

A Shared Work Queue accepts work submission using the PCIe-defined Deferrable Memory Write Request (DMWr). DMWr is a 64-byte non-posted write that waits for a response from the device before completing. The device returns Success if the descriptor is accepted into the work queue, or Retry if the descriptor is not accepted due to WQ capacity or QoS. This allows multiple clients to directly and simultaneously submit descriptors to the same work queue. Since the device provides this feedback, the clients can tell whether their descriptors were accepted. On Intel CPUs, DMWr is generated using the ENQCMD or ENQCMDS instructions. The ENQCMD and ENQCMDS instructions return the status of the command submission in EFLAGS.ZF flag; 0 indicates Success, and 1 indicates Retry.

A Shared WQ can be configured to reserve some of the WQ capacity by setting the WQ Threshold field in the WQCFG register. Work submission via a limited portal is accepted until the number of descriptors in the SWQ reaches the configured threshold. Work submission via an unlimited portal is accepted unless the SWQ is completely full. The unlimited portals are intended to be used only by privileged software when a work submission to the corresponding limited portal returns Retry. User-mode and guest software typically only have access to limited portals.

If DMWr returns Success, the descriptor has been accepted by the device and queued for processing. If DMWr returns Retry, software can try re-submitting the descriptor to the SWQ, or if it was a user-mode client using a limited portal, it can request that the kernel-mode driver submit the descriptor on its behalf using an unlimited portal. This helps avoid denial of service and provide forward progress guarantees. See chapter 7 for more information on software use of the limited and unlimited portals.

Clients are identified by the device using a 20-bit ID called Process Address Space ID (PASID). The PASID capability must be enabled to use SWQs. The PASID is used by the device to look up addresses in the Address Translation Cache and to send address translation or page requests to the IOMMU. In Shared mode, the PASID to be used with each descriptor is contained in the PASID field of every descriptor. The ENQCMD instruction copies the PASID of the current thread from the IA32_PASID MSR into the descriptor while ENQCMDS allows supervisor mode software to copy the PASID into the descriptor. For additional details on the use of PASID and the ENQCMD and ENQCMDS instructions, refer to the Intel® Architecture Instruction Set Extensions Programming Reference, listed in the References in section 1.2.

# 3.3.2 Dedicated Work Queue (DWQ)

To submit work to a Dedicated Work Queue, software uses a 64-byte memory write transaction with write atomicity. This transaction may complete faster than DMWr due to the posted nature of the write operation. The device depends on software to provide flow control based on the number of slots in the work queue. Software is responsible for tracking the number of descriptors submitted and completed, to detect a work queue full condition. If software erroneously submits a descriptor to a dedicated WQ when there is no space in the work queue, the descriptor is dropped. (The error is reported in the Software Error Register.)

On Intel CPUs, work submission to a DWQ is performed using the MOVDIR64B instruction, which generates a non-torn 64-byte write. For information about the MOVDIR64B instruction, refer to the Intel® 64 and IA-32 Architectures Software Developer's Manuals, listed in the References in section 1.2.

With dedicated WQs, the use of PASID is optional. If the PCI Express PASID capability is not enabled, PASID is not used. If the PASID capability is enabled, the WQ PASID Enable field of the WQ Configuration register controls whether PASID is used for each DWQ. Since the MOVDIR64B instruction does not fill in the PASID as the ENQCMD or ENQCMDS instructions do, the PASID field in the descriptor is ignored. When PASID is enabled for a DWQ, the device uses the WQ PASID field of the WQ Configuration register to do address translation. The WQ PASID field must be set by the driver before enabling a work queue in dedicated mode.

Although dedicated mode doesn't support the sharing of a single DWQ by multiple clients, Intel DSA can be configured to have multiple DWQs and each of the DWQs can be independently assigned to clients. DWQs can be configured to have the same or different QoS levels.

# 3.4 Engines and Groups

An engine is an operational unit within an Intel DSA device. A group is a set of work queues and engines. Software configures WQs and engines into groups using the Group Configuration registers. Each group contains one or more WQs and one or more engines. Any engine in a group may be used to process a descriptor posted to any WQ in the group. Each WQ and each engine may be in only one group.

Although the Intel DSA architecture allows great flexibility in configuring work queues, groups, and engines, the hardware is designed with the intent to be used in specific configurations. Example configurations are shown in Figures 3-2 and 3-3. In the configuration shown in Figure 3-2, hardware uses either engine in a group to process descriptors from any work queue in the group. If one engine has a stall due to a high-latency memory address translation or page fault, the other engine can continue to operate and maximize the throughput of the overall device.

Figure 3-2 shows example Traffic Class (TC) values for the two groups. In Group 0 both TC values are 0, while in Group 1, TC-B is 1. This example configuration might be used when Group 0 is used solely for operations that access DRAM, and Group 1 is used for operations that access both DRAM and persistent memory. The TC Selector flags in descriptors submitted to Group 1 indicate whether each address in the descriptor refers to DRAM or persistent memory. See chapter 4 for information on Traffic Classes and how they can be used to control QoS.

Figure 3-2 shows two work queues in each group, but there may be any number up to the maximum number of WQs supported. The WQs in a group may be shared WQs with different priorities, or one shared WQ and the others dedicated WQs, or multiple dedicated WQs with the same or different priorities.

Figure 3-3 shows another example configuration, in which each engine is placed in a separate group. Software may choose this configuration when it wants to reduce the likelihood that latency-sensitive operations become blocked behind other operations. In this configuration, software submits latency-sensitive operations to the work queue connected to one engine, and other operations to the work queues connected to another engine. If the group used for latency sensitive operations is idle when a descriptor is submitted, the descriptor will be dispatched to an engine immediately.

![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/cf421822d02a2edc078f496b19c5ba7c0478eec0c0352bbda2bdb42799204218.jpg)



Figure 3-2: Sample Group Configuration 1


![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/12265fca9cac96d7edcbf580adbd6eaa77f4d8622809fceafc9b1da9ec1fed81.jpg)



Figure 3-3: Sample Group Configuration 2


Software can also mix these two, with some engines in a single group and the others in groups by themselves.

# 3.5 Descriptor Processing

As each descriptor reaches the head of the work queue, it is available to be dispatched by the group arbiter to an available engine in the group. The arbiter for each group dispatches descriptors from the WQs in the group according to their priority, while ensuring that the higher priority WQs don't starve lower priority WQs. See section 4.1 for information about work dispatch priority.

For a Batch descriptor, which refers to work descriptors in memory, the engine fetches the array of work descriptors from memory. Each work descriptor is passed to the work descriptor processing unit. The work descriptor processing unit uses the Address Translation Cache and IOMMU for completion record, source, and destination address translations; reads source data; performs the specified operation; and writes the destination data back to memory. When the operation is complete, the engine writes the completion record to the pre-translated completion address and generates an interrupt, if requested by the work descriptor.

# 3.6 Descriptor Completion

Descriptors contain three flags and two other fields that allow software to control completion notifications. The three flags are: Completion Record Address Valid, Request Completion Record, and

Request Completion Interrupt. The two fields are Completion Record Address and Completion Interrupt Handle.

The completion record is a 32-byte aligned structure in memory that the device writes when the operation is complete or encounters an error. The completion record contains completion status. If the operation completed successfully, the completion record may contain the result of the operation, if any, depending on the type of operation. If the operation did not complete successfully, the completion record contains fault or error information.

Generally, all descriptors should have a valid Completion Record Address and the Completion Record Address Valid flag should be 1. (Exceptions to this rule are described later.)

The first byte of the completion record is the status byte. Status values written by the device are all nonzero. Software should initialize the status field of the completion record to 0 before submitting the descriptor to be able to tell when the device has written to the completion record. (Initializing the completion record also ensures that it is mapped, so the device is less likely to encounter a page fault when accessing it.)

The Request Completion Record flag indicates to the device that it should write the completion record even if the operation completed successfully. If this flag is not set, the device writes the completion record only if there is an error.

Descriptor completion can be detected by software using any of the following methods:

1. Poll the completion record, waiting for the status field to become non-zero.

2. Use the UMONITOR/UMWAIT instructions on the completion record address to block until it is written or until timeout. Software should then check whether the status field is non-zero to determine whether the operation has completed.

3. Request an interrupt when the operation is completed. For user-mode descriptors, this method requires the kernel to forward the notification to the application.

4. If the descriptor is in a batch, set the Fence flag in a subsequent descriptor in the same batch. Completion of the descriptor with the Fence or any subsequent descriptor in the same batch indicates completion of all descriptors that precede the Fence.

5. If the descriptor is in a batch, completion of the Batch descriptor that initiated the batch indicates completion of all descriptors in the batch.

6. Issue a Drain descriptor or a Drain command and wait for it to complete.

If the completion status indicates a partial completion due to a page fault, the completion record indicates how much processing was completed (if any) before the fault was encountered, and the virtual address where the fault was encountered. Software may choose to fix the fault (by touching the faulting address from the CPU) and resubmit the rest of the work in a new descriptor or complete the rest of the work in software. Faults on descriptor list and completion record addresses are handled differently and are described in more detail in section 3.13.

# 3.7 Interrupts

Intel DSA supports only message signaled interrupts. It provides two types of interrupt message storage: (1) an MSI-X table, enumerated through the MSI-X capability; and (2) a device-specific Interrupt Message Storage (IMS) table, as described by the Intel Scalable IOV Architecture Specification. For more information on IMS, refer to section 9.2.28, and to the Intel® Scalable I/O Virtualization Technical Specification, listed in the References in section 1.2.

Interrupts can be generated for six types of events: 1) completion of a descriptor; 2) WQ occupancy below programmed limit; 3) completion of an administrative command; 4) an error posted in the Software Error Register or written to the Event Log in memory<sup>1</sup>; 5) performance monitoring counter overflow; and 6) interrupt handle revocation. For each type of event, there is a separate interrupt enable. Interrupts for types 3-6 are generated using entry 0 in the MSI-X table. The Interrupt Cause Register may be read by software to determine the reason for the interrupt.

For completion of a descriptor that requests a completion interrupt, the interrupt message used is dependent on the portal the descriptor was submitted to and the Completion Interrupt Handle in the descriptor. As described in section 3.3, each WQ has both MSI-X portals and IMS portals. For a descriptor submitted via an MSI-X portal, the Completion Interrupt Handle field in the descriptor selects an entry in the MSI-X table. For a descriptor submitted via an IMS portal, the Completion Interrupt Handle field in the descriptor selects an entry in the Interrupt Message Storage. Descriptors in a batch are treated as if they had been submitted via the same portal as the Batch descriptor.

<table><tr><td>Event</td><td>Submission Register</td><td>Interrupt Message Used</td></tr><tr><td>Error posted in SWERROR register or written to the Event Log</td><td>N/A</td><td>MSI-X table entry 0.</td></tr><tr><td>Completion of an administrative command</td><td>Command register</td><td>MSI-X table entry 0.</td></tr><tr><td>Perfmon counter overflow</td><td>N/A</td><td>MSI-X table entry 0.</td></tr><tr><td>WQ Occupancy below limit</td><td>WQ Occupancy Interrupt register</td><td>MSI-X or IMS entry programmed in WQ Occupancy Interrupt register.</td></tr><tr><td rowspan="2">Descriptor completion</td><td>MSI-X portal</td><td>MSI-X table entry specified by Completion Interrupt Handle field in descriptor.</td></tr><tr><td>IMS portal</td><td>Interrupt Message Storage entry specified by Completion Interrupt Handle field in descriptor.</td></tr><tr><td>Interrupt handle revocation</td><td>N/A</td><td>MSI-X table entry 0.</td></tr></table>

Table 3-1: Interrupt Delivery

When the Request Interrupt Handle command is not supported (as indicated by the Command Capabilities register), the Completion Interrupt Handle is the index of the desired entry in the MSI-X table or the IMS. When the Request Interrupt Handle command is supported, software must use the command to obtain a handle to use for the interrupt. Software specifies in the Request Interrupt Handle command which interrupt table entry it wants a handle for. The response to the command contains the handle that software should place in the Completion Interrupt Handle field of the descriptor to request that interrupt.

An interrupt handle obtained using the Request Interrupt Handle command may be revoked. After an interrupt handle is revoked, any use of the handle will result in an Invalid Interrupt Handle error. When one

or more interrupt handles are revoked, the device sets the Interrupt Handles Revoked bit in the Interrupt Cause register and generates an interrupt using MSI-X table entry 0. This interrupt cause can only occur if the Request Interrupt Handle command has been used to obtain interrupt handles. Software should use the Request Interrupt Handle command to obtain new handles for all MSI-X and/or IMS entries in use. Software should then resubmit any descriptors that failed with an Invalid Interrupt Handle error using the new handles. See section 7.5.4 for a description of interrupt virtualization including details of the steps software should perform to support interrupt handle revocation.

The MSI-X table defined by the PCIe specification is augmented in Intel DSA by the MSI-X Permissions Table, detailed in section 9.2.22. Each MSI-X Permissions Table entry has several fields that control generation of interrupts using that table entry. Each IMS entry contains the same control fields. The PASID Enable and PASID fields of the selected interrupt table entry are checked before the descriptor is executed, as detailed in section 5.4. The Ignore and Mask fields are checked when the descriptor completes. If the Ignore field is 1, no interrupt is generated. If the Ignore field is 0, the Mask and Pending fields behave as specified by PCIe. If the Mask field is 1, the Pending field is set to 1 and no interrupt is generated. If Ignore and Mask are both 0, the interrupt is generated. For interrupts other than descriptor completions, the PASID Enable, PASID, and Ignore fields are not used.

Interrupts generated by Intel DSA are processed through the Interrupt Remapping and Posting hardware as configured by the kernel or VMM software.

# 3.8 Batch Descriptor Processing

Intel DSA supports submitting multiple descriptors at once. A Batch descriptor contains the address of an array of work descriptors in host memory and the number of elements in the array. The array of work descriptors is called the "batch." Use of Batch descriptors allows software to submit multiple work descriptors using a single work submission operation and can potentially improve overall throughput, especially when using descriptors with small transfer sizes.

Intel DSA enforces a limit on the number of work descriptors in a batch. There is an overall limit, indicated by the Maximum Supported Batch Size field in the General Capabilities register, and also a separate limit for each work queue, set by the WQ Maximum Batch Size field for each WQ in the WQ Configuration Table. A batch must contain at least 2 work descriptors, unless the Batch1 Support field in GENCAP is 1, indicating that a batch containing only one descriptor is supported.

Batch descriptors are submitted to work queues in the same way as other work descriptors. When a Batch descriptor is processed by the device, the device reads the array of work descriptors from memory and then processes each of the work descriptors. The work descriptors are not necessarily processed in order. (See section 3.9 for information on how software can control ordering of descriptors in a batch.)

The PASID and the Priv fields of a Batch descriptor are used for all descriptors in the batch. The PASID and Priv fields in the descriptors in a batch are ignored.

Each work descriptor in a batch can specify a completion record address and/or a completion interrupt, just as with directly submitted work descriptors. The completion record and completion interrupt for the

Batch descriptor (if requested) are generated after completion of all the descriptors in the batch and generation of their completion records (if requested). No readback is performed before the Batch descriptor completion record is generated. To maintain ordering of the completion record for the Batch behind all writes from descriptors in the batch, either the Batch descriptor should use the same TC for its completion record as the prior writes, or each of the descriptors in the batch must specify destination readback using the cache control flags, as described in section 3.9. To maintain ordering of the completion record for the Batch after the completion records of the descriptors in the batch, the same TC should be used for all of the completion records.

The completion record for the Batch descriptor contains an indication of whether any of the descriptors in the batch completed with Status not equal to Success. This allows software to avoid examining all the completion records for the descriptors in the batch, in the usual case where all the descriptors in the batch completed successfully. In some cases, if a descriptor in the batch encountered a page fault on the completion record address, hardware may indicate a possible error even though the completion record page fault was resolved successfully. Software can examine the completion records for descriptors in the batch to determine whether there were truly any failures.

A Batch descriptor may not be included in a batch. Nested or chained descriptor arrays are not supported. See section 8.3.2 for details on the format of Batch descriptors.

# 3.9 Ordering and Fencing

Descriptors may generally be processed by the device in any order. However, descriptors are guaranteed to be executed in the order that they are received by the device when all of the following conditions are met:

- Descriptors are submitted to a group with only one engine.

- Descriptors are all submitted to the same WQ using the same portal address.

- Descriptors are all Batch descriptors, or they are all not Batch descriptors.

- Descriptors all use the same Destination TC Selector.

Only write ordering is guaranteed. Reads by a subsequent descriptor can pass writes from a previous descriptor. If an error occurs in a descriptor, subsequent descriptors will continue to execute. Thus, software cannot necessarily rely on data transfers from earlier descriptors completing before those from later descriptors. The order in which completion records become visible to software is not guaranteed.

Even when these conditions are met, the order of descriptors within a batch is not guaranteed unless the Fence flag is set as described below.

If more control of the ordering of descriptors is required, software may use one of the following methods:

- Submit a descriptor, wait for the completion record or interrupt from the descriptor to ensure completion, and then submit the next descriptor.

- Use a Drain descriptor or Drain command to wait for preceding descriptors to complete, and then submit the following descriptors.

- Within a batch, use the Fence flag.

Enforcing ordering may increase both the CPU time used to submit a descriptor and the latency for the descriptor to begin execution within the device.

To control ordering for descriptors in a batch specified by a Batch descriptor, each work descriptor has

a Fence flag. When set, Fence guarantees that processing of that descriptor will not start until all previous descriptors in the same batch are completed. This allows a descriptor with Fence to consume data produced by a previous descriptor in the same batch. A descriptor consuming data from a previous descriptor in the batch should use the same Traffic Class as the descriptor producing the data. If software cannot ensure this, then the descriptor that produces the data must specify destination readback using the cache control flags, as described below, in order to ensure the required ordering.

If any descriptor in a batch completes with Status not equal to Success, for example if it is partially completed due to a page fault, a subsequent descriptor with the Fence flag equal to 1 and any following descriptors in the batch are abandoned. The completion record for the Batch descriptor that was used to submit the batch indicates how many descriptors were processed prior to the Fence.

The completion record write for a descriptor is ordered after all data writes produced by the descriptor if:

- the descriptor is fully completed; or

- the completion record TC Selector in the descriptor is the same as the destination TC Selector(s).

Otherwise, the completion record may be observed by software before some of the data writes produced by the descriptor. A completion interrupt (if requested) is ordered after the completion record write.

If a Batch descriptor does not request a completion record (e.g., Completion Record Address Valid is 0 or Request Completion Record is 0), the ordering of the completion interrupt for the Batch descriptor (if requested) relative to completion record writes for descriptors in the batch is not guaranteed if the completion record writes specify a TC Selector that selects a non-zero TC. In this case, software can set the Request Completion Record flag in the Batch descriptor to ensure correct ordering of the completion interrupt for the Batch descriptor.

Certain combinations of the cache control flags in the descriptor specify destination readback, as described in section 8.1.3.1. When a descriptor specifies destination readback, it causes Intel DSA to perform a zero-length read, using the final destination address of the descriptor, prior to writing the completion record. If the destination target is different from the completion record target, then the destination readback may be specified to ensure that writes have propagated to the destination before the completion record is written. For example, this flag may be used in descriptors that target NTB to ensure that data written by the descriptor has propagated across the NTB link. Destination readback is performed only if the descriptor is completed successfully. If the descriptor is partially completed, the readback is not performed. If a follow-up descriptor to complete the operation writes to the same destination using the same TC, specifies destination readback, and completes successfully, then the readback performed by the follow-up descriptor also ensures completion of memory writes performed by the prior descriptor(s). See section 3.10 for more information on the cache control flags.

# 3.10 Cache Control

The cache control flags in the descriptor provide a hint with respect to data placement of memory writes during descriptor processing. The three flags are Cache Control 1, Cache Control 2 $^1$ , and Cache Control 3. These flags are described in section 8.1.3. The hint indicates whether the data produced by the descriptor should be written to a last level cache or to memory. In system configurations with multiple last level caches, the hint may allow software to indicate a preferred cache for data placement. The hint does not affect writing to the completion record, which is always directed to the cache hierarchy. The various flag combinations and the corresponding data placement hints are described in section 8.1.3.1. The table also indicates the combinations of flags that are reserved.

The hint may be ignored by an implementation. Because processors are free to speculatively fetch data into the caches or evict data from the caches at any time, the effect of these flags are not guaranteed, even when they are supported.

These flags are reserved for operations that do not write to memory. Certain flag combinations are supported for the Cache Flush operation as described in section 8.3.26.

# 3.11 Persistent Memory Support

Intel DSA provides the ability to ensure that data written by a descriptor has become persistent at the time the descriptor completes. The cache control flags in the descriptor provide a hint to indicate whether writes should be directed to cache or to memory and also whether the write is directed to persistent memory. Section 8.1.3.1 describes which values of the cache control flags indicate durable writes.

If Durable Write Support in GENCAP is 0, write persistency may be specified by setting the Cache Control 1 flag in a descriptor to 0 and setting the Cache Control 2 flag to 1. This also causes a readback to the destination address when the operation is complete.

If Durable Write Support in GENCAP is 1, write persistency may be specified by setting Cache Control 3 flag to 1 and both Cache Control 1 and Cache Control 2 flags to 0. In this case, no readback is performed.

As described in section 2.2.1, since completion record writes are always directed to cache, there is no device-supported mechanism to ensure persistence of completion record writes.

The Strict Ordering flag does not guarantee that writes become persistent in order.

# 3.12 Drain Descriptor

A Drain descriptor waits for completion of certain preceding descriptors in the WQ that the Drain descriptor is submitted to. If a Drain descriptor is submitted to a dedicated WQ, it waits for completion of all descriptors in the WQ. If a Drain descriptor is submitted to a shared WQ, it waits for descriptors in the WQ that were submitted with the same PASID as the Drain descriptor. To wait for all descriptors with a particular PASID, software should submit a separate Drain descriptor to every WQ that the PASID was

used with. To wait for all descriptors in a WQ regardless of PASID, software may use the Drain WQ command described in section 3.13.3.

A Drain descriptor may be used during normal shutdown by a process that has been using the device. It can be used like a Fence operation for the entire PASID. It can be used to request a single completion record and/or interrupt for the completion of multiple descriptors. A Drain descriptor may not be included in a batch. (A Fence flag may be used in a batch to wait for prior descriptors in the batch to complete.) Software should execute a fencing instruction such as SFENCE or MFENCE before submitting a Drain descriptor to ensure that the Drain descriptor is received by the device after the descriptors it is intended to drain.

For the purpose of Drain, a preceding descriptor is completed after all writes generated by the operation are globally observable; after destination readback, if requested; after the write to the completion record is globally observable, if needed; and after generation of the completion interrupt, if requested. To ensure this, prior to Drain descriptor completion, hardware normally issues an implicit readback for each supported Traffic Class using an address determined by hardware. The implicit readbacks ensure that all previous writes to the Root Complex have completed.

Software can control the default behavior by setting readback flags in the Drain descriptor that can be used to suppress the implicit readbacks and/or request explicit readbacks to software-controlled addresses. Previous writes to a peer device (i.e., non-Root Complex) may not be flushed by a Drain descriptor implicit readback but can be flushed using explicit readbacks. The Drain descriptor allows software to specify up to 2 explicit readback addresses in the descriptor. If specified, hardware will issue readbacks to each explicit readback address using the Traffic Class specified by the corresponding TC Selector flag in the descriptor. See section 8.3.3 for details of the Drain descriptor.

# 3.13 Address Translation

Intel DSA supports the use of either physical or virtual addresses. The use of virtual addresses that are shared with processes running on the CPU is called shared virtual memory (SVM). To support SVM, the device provides a PASID when performing address translations and it handles page faults that occur when no translation is present for an address. However, the device itself doesn't distinguish between virtual and physical addresses; this distinction is controlled by the programming of the IOMMU.

Intel DSA supports the PCI Express Address Translation Service (ATS) and Page Request Service (PRS) capabilities. ATS describes the device behavior during address translation. When a descriptor enters a descriptor processing unit, the device requests translations for the addresses in the descriptor. If there is a hit in the Address Translation Cache, the device uses the corresponding HPA. If there is a miss or permission fault, the device sends an address translation request to IOMMU for the translation. The IOMMU finds the translation by walking the appropriate page tables and returns an address translation response that contains the translated address and the effective permissions. The device stores the translation in the Address Translation Cache and uses the corresponding HPA for the operation. If IOMMU can't find the translation in the page tables, it returns an address translation response that indicates no translation is available. When the IOMMU response indicates no translation or indicates effective permissions that don't include the permission required by the operation, it is considered a page fault.

# 3.13.1 Work Queue Address Translation Controls

Some implementations support the ability to control ATS and PRS capabilities for each enabled work queue in the device. Support for work-queue granular ATS and PRS controls is indicated by the WQ ATS Support and WQ PRS Support fields in WQCAP. If supported, software can use the WQ ATS Disable and WQ PRS Disable controls in WQCFG to disable the corresponding capabilities for each work queue. The device operation for descriptors submitted to a work queue that has either one of the capabilities disabled is the same as when the corresponding PCI Express control is 0.

If ATS is disabled, either by the PCIe ATS Control register or by the WQ ATS Disable flag in WQCFG, Intel DSA does not receive any notification of address translation faults for memory write transactions. The completion record for an operation may indicate success even if a page fault occurred for a write transaction. If ATS is disabled, software should ensure that no memory addresses accessed by a descriptor can cause a page fault. Software may use the cache control flags to detect some errors writing to the destination address. If a destination readback is specified and the readback fails, that indicates that the write may not have succeeded.

# 3.13.2 Page Fault Handling with PRS Enabled

PRS is enabled when the PCI Express PRS Enable Control is 1 and either WQ PRS Support in WQCAP is 0 or WQ PRS Disable in WQCFG is 0. In addition, each descriptor has a Block On Fault flag that specifies whether PRS is enabled for the source and destination buffer addresses in the descriptor.

When a page fault occurs and PRS is enabled, the fault is reported as a PRS request to the IOMMU for servicing by the OS page fault handler. The IOMMU notifies the OS through an interrupt. The OS validates the address and upon successful checks, creates a mapping in the page table and returns a PRS response through the IOMMU. The descriptor encountering the fault is blocked until the PRS response is received. Other operations behind the descriptor with the fault may also be blocked. If the OS was not able to create a mapping, it returns an error response and the descriptor is completed with an error. The error reporting is the same as page fault reporting when PRS is disabled, described in the next section.

When Block On Fault is 0 and a page fault is encountered on a source or destination buffer address, the device stops the operation and writes the partial completion status along with the faulting address and progress information into the completion record. (See sections 8.1 and 8.1.9 for more details.) The Block On Fault Support field in the General Capabilities register (GENCAP) indicates device support for enabling PRS for source and destination buffers, and the Block On Fault Enable field for each WQ in the WQ Configuration Table allows the VMM or kernel driver to control which applications are allowed to enable PRS for source and destination buffers. These registers are described in section 9.1.4.

Device page faults are relatively expensive, higher than the cost of servicing CPU page faults. Hence, for best performance, it is desirable for software to minimize device page faults without incurring the overheads of pinning and unpinning.

Batch descriptor lists and source data buffers are typically produced by software right before submitting them to the device. Hence, these addresses are not likely to incur faults due to temporal locality. Completion records and destination data buffers, however, are more likely to incur faults if they are not

touched by software before submitting to the device. Such faults can be minimized by software explicitly "write touching" these pages before submission.

# 3.13.3 Page Fault Handling with PRS Disabled

PRS is disabled for all address translations when the PCI Express PRS Enable Control is 0 or the WQ PRS Disable field in WQCFG is 1. PRS is disabled only for the source and destination buffer addresses in a descriptor when the Block on Fault flag in the descriptor is 0.

When a page fault occurs on a Completion Record Address, the error is reported in the Event Log, if enabled, or in the SWERROR register. Completion record faults reported in the event log are recoverable. The descriptor is completed and the event log entry contains the completion record address and the contents of the completion record. After handling the page fault, system software should copy the completion record to the completion record address. See section 5.9 for information about the event log and section 7.3 for information about software handling of completion record faults reported in the event log. Completion record faults reported in SWERROR may not be recoverable: faults may be lost since only a single error can be logged; and the descriptor is not completed and must be restarted, since partial completion information is not recorded.

When a page fault occurs for an address in a descriptor other than a Completion Record Address, the device stops the operation and writes the partial completion status along with the faulting address and progress information into the completion record. See sections 8.1 and 8.1.9 for more details. When the client software receives a completion record indicating partial completion, it has the option to fix the fault on CPU (by touching the page, for example) and submit a new work descriptor with the remaining work. Alternatively, software can complete the remaining work on the CPU.

Operating with PRS disabled and Event Log enabled allows applications using Intel DSA to avoid having significant impact on other applications in the presence of page faults. Device page faults can be expensive even when PRS is disabled because they require software intervention to service the page fault and resubmit the work. It is desirable for software to minimize device page faults as described in the previous section.

# 3.14 Inter-Domain Operations

Every descriptor submitted to Intel DSA is associated with a default address space, which corresponds to the address space of the work submitter. If the PASID capability is enabled, the default address space is specified by the PASID carried in the work descriptor submitted to a shared work queue, or by the PASID configured in the WQCFG register for a dedicated work queue (as described in section 9.2.24). Memory accesses and translation requests are tagged with this PASID value. If the PASID capability is not enabled, the default address space is implicitly specified to the IOMMU via the PCIe requester ID (bus, device, function) of the device.

The default address space of a descriptor, represented by the descriptor PASID, is normally used for memory accesses and IOMMU requests from the device. If the PASID capability is enabled, certain operations allow the submitter to select an alternate address space for either the source addresses, destination addresses, or both source and destination addresses specified in the descriptor. The

alternate address space is usually that of a cooperating process<sup>1</sup>. The term domain is used to denote an address space, and an operation that selects an alternate address space for any of its addresses is an inter-domain operation.

Support for inter-domain operations is indicated by the Inter-Domain Support field in the GENCAP register (described in section 9.2.2). When this field is 1, inter-domain capabilities are reported in the IDCAP register (described in section 9.2.18). The set of inter-domain operations supported by an implementation is reported in the OPCAP register. For certain operation types, the Operations with Inter-Domain Support field in DSACAP0 indicates whether inter-domain support is available for each operation type. Details of operations with inter-domain support, along with a description of the corresponding descriptor fields, can be found in chapter 8.

Inter-domain operations require the PASID capability to be enabled. Selection of PASIDs used in each operation is done using appropriate descriptor fields.

If a work submitter does not explicitly select an alternate PASID for an address in a descriptor, the descriptor PASID is used for memory accesses and translation requests pertaining to that address. If a descriptor selects an alternate PASID for an address, called the Access PASID, it is used instead of the descriptor PASID, if the submitter has appropriate permissions to do so. The device uses the access PASID to perform memory accesses and translation requests pertaining to that address. The descriptor PASID is always used to write the completion record, to read the SGL, to generate interrupts, and to verify that a submitter has adequate permissions to the specified access PASID, as described below.

An inter-domain operation may involve two or three PASIDs depending on the use case. Some of the typical use cases are listed below.

1. Data read or write by one or more user-mode submitters from or to a memory region exported by a user-mode owner.

2. Data read or write by a kernel-mode submitter from or to a memory region of a user-mode process.

3. Data read or write by a kernel-mode submitter between memory regions of two distinct user-mode processes.

4. Data read or write by a kernel-mode submitter from or to a memory region of another kernel-mode process.

5. Data read or write by a privileged submitter between memory regions of two distinct guest OSes.

6. Any of the above executed within a guest OS.

Use case (1) above requires an owner to explicitly grant access to a portion of its memory space to one or more submitters. The memory region that an owner grants access to is referred to as a memory window. A memory window is only accessible using the owner's PASID as the access PASID. Use cases (2) to (6) involve privileged software accessing memory regions of other user-mode or kernel-mode processes within that OS domain.

# 3.14.1 Inter-Domain Permissions Table (IDPT)

If inter-domain operations are supported, Intel DSA implements an Inter-Domain Permissions Table to allow software to manage 1) the association between a descriptor PASID and an access PASID that a work submitter is allowed to access; 2) attributes of a memory region in an access PASID's memory space that a submitter is allowed to access; and 3) controls to manage the life cycle of such association. The Inter-Domain Permissions Table is managed by the host kernel-mode driver and may be configured to support uses for both kernel-mode and user-mode applications, in a host or guest OS.

Each entry in the IDPT contains the following:

1. An entry type as described below.

2. One or more submitter PASID values allowed to use that entry and a mechanism to validate them.

3. Depending on the entry type, an access PASID to be used for memory accesses.

4. Memory window address range and attributes.

5. Permissions and other control information.

Each Inter-Domain Permissions Table entry (IDPTE) may be configured in one of the following ways as indicated by the Type field described below and summarized in section 9.2.29):

- Type 0-Single Access, Single Submitter (SASS): The IDPTE specifies a single access PASID and a single submitter PASID. For example, a process that wants to expose a memory window to a peer process may request the driver to set up a SASS entry with its own PASID as the access PASID and the PASID of its peer as the submitter PASID.

- Type 1-Single Access, Multiple Submitter (SAMs): The IDPTE specifies a single access PASID. The submitter PASID field in the entry is unused. Instead, the IDPTE points to a bitmap in memory that specifies the set of submitter PASIDs allowed to use the entry. A bit set to 1 in the bitmap indicates that the corresponding PASID is allowed to submit an inter-domain operation using the IDPTE. For example, a process that wants to allow multiple submitters to access a window in its address space requests a SAMs entry to be set up.

<table><tr><td>Type</td><td>Mnemonic</td><td>Description</td><td>Access PASID Obtained From</td><td>Submitter PASID Matched Against</td></tr><tr><td>00</td><td>SASS</td><td>Single Access, Single-submitter entry (1 access PASID, 1 submitter PASID)</td><td>IDPT entry</td><td>IDPT entry</td></tr><tr><td>01</td><td>SAMS</td><td>Single Access, Multi-submitter entry (1 access PASID, N submitter PASIDs)</td><td>IDPT entry</td><td>Bitmap</td></tr></table>

Figure 3-4: Inter-Domain Permissions Table Entry Types

A descriptor refers to an IDPTE entry using a handle in the descriptor. The Inter-Domain Permissions Table Size field in IDCAP specifies the maximum size of the IDPT table supported by the device. Software allocates IDPT entries with index less than this maximum size. In addition, if the Request IDPT Handle field in CMDCAP is 0, the handle is the index of the desired entry in the IDPT. If the Request IDPT Handle field in CMDCAP is 1, software must use the Request IDPT Handle command to obtain the

handle to use corresponding to the allocated index in the IDPT. Software specifies in the Request IDPT Handle command the index of the PASID table entry it wants a handle for, and the response to the command contains the handle that software should place in the descriptor. See section 7.5.6 for a description of IDPT virtualization.

An inter-domain descriptor may contain more than one handle, depending on the type of operation. A separate handle may be specified for each distinct source and/or destination address in a descriptor. Each handle in a descriptor is used by hardware to look up the corresponding IDPTE to 1) validate access permissions for the submitter, 2) identify the access PASID and privilege to be used for memory access, 3) compute the effective memory address, and 4) verify that the access conforms to the memory window and permissions granted by the IDPTE.

An IDPTE may be referenced by:

- An inter-domain descriptor while the Usable bit in the IDPTE is 1. The hardware checks that the descriptor PASID matches a submitter PASID value in the specified IDPTE.

- An Update Window descriptor while the Allow Update bit in the IDPTE is 1. The hardware checks that the descriptor PASID matches the access PASID value in the specified IDPTE.

If the PASID values do not match, then memory accesses using that entry are disallowed for that descriptor, and the descriptor is completed with an error.

Details of the operations are described in chapter 8.

# 3.14.2 Submitter Bitmap

As mentioned above, a type 1 SAMS IDPTE points to a submitter bitmap in memory, with one bit for every possible PASID value. The bitmap is indexed by the PASID value to be checked against the bitmap. Access is allowed only if the bit corresponding to the checked PASID is 1 in the bitmap. For a SAMS IDPTE, hardware checks the descriptor PASID against the bitmap prior to allowing any memory access using the table entry. A type 1 SAMS entry specifies a 4KB aligned virtual or physical address, referred to as the Submitter Bitmap Address. Privileged software like the kernel-mode driver is responsible for setting up and maintaining the bitmap in memory. The maximum size of a submitter bitmap is  $2^{20}$  bits, i.e., 128KB. Each IDPTE that requires a bitmap may point to a distinct submitter bitmap in memory. Software may also choose to share a submitter bitmap between multiple IDPTEs, if appropriate.

The IDBR (described in section 9.2.19) controls whether hardware should use a PASID value for submitter bitmap reads. If enabled, the IDBR specifies the PASID value and privilege to be used for bitmap reads. Although each submitter bitmap is mapped to a contiguous virtual address range in the corresponding PASID space, it may be mapped into discontinuous physical pages in system memory. Software is also not required to map the bitmap entirely into system memory at a given time; different bitmap pages may be mapped as needed. If a page of a bitmap is inaccessible, all bits on that page are treated as  $0$ . The IDBR also specifies the traffic class to be used for bitmap reads.

Bitmap reads may be done by hardware in an implementation-specific manner. An implementation may issue bitmap reads as either Translated or Untranslated accesses. Hardware may read a single byte or dword or a cache line or larger region of a bitmap, corresponding to the PASID to be checked. For example, for a PASID value of p to be checked against a bitmap, an implementation that uses cache line reads of the bitmap would read the cache line at (Submitter Bitmap Address + ((p >> 3) & 0xFFFFFFFCO)) and examine the bit corresponding to the PASID to be checked.

An implementation is allowed to cache portions of a bitmap in order to avoid repeated memory reads. This capability, if supported, is indicated by the InvalidateSubmitterBitmapCachefieldinCMDCAP.If this capability is 1, software must issue the InvalidateSubmitterBitmapCache command (described in section 9.2.12) after it modifies any portion of a bitmap in memory or modifies the mapping of any page of the bitmap. In the latter case, software must perform the bitmap invalidation after it performs any required invalidations normally associated with page mapping modifications.

# 3.14.3 Memory Window

A memory window is a region of memory in an owner's address space that it allows one or more submitters to access. It is defined by the window base address, window size, window mode, and access permissions fields in the IDPTE. The window attributes are initialized at the same time that an IDPTE is allocated by the kernel-mode driver to an owner or to a privileged submitter. The Window Enable field in an IDPTE controls whether a memory window is active for that IDPTE.

If Window Enable is 0, hardware does not perform an address range check when using that entry. A validated submitter is allowed to access any address in the address space, and the Window Mode, Window Base, and Window Size fields are reserved.

If Window Enable is 1, hardware checks that the memory region in a descriptor referencing the IDPTE falls within the memory window. The memory window must not wrap around the  $2^{64}$  address boundary. The Window Mode field controls the interpretation of the address in a descriptor referencing the IDPTE.

# Two window modes are supported:

- Address Mode: Hardware checks that the memory region in the descriptor that references the IDPTE lies within the window, i.e., between the window base address and the sum of window base address and window size.

- Offset Mode: The address of the memory region in the descriptor is treated as an offset from the window base address. The effective start of the memory region is computed as the sum of the window base address and the address in the descriptor referencing that IDPTE. The effective end of the memory region is the sum of the effective start address and region size. The effective start and end of the memory region must lie within the window.

An IDPTE specifies read and write permissions for memory accesses using that entry. If the requested permissions do not match the granted permissions, the access is denied.<sup>1</sup>

# 3.14.4 Inter-Domain Completions

If an inter-domain operation is partially completed due to a page fault in an alternate PASID address space, or if the operation fails because of an invalid IDPT handle, a misconfigured IDPTE, or insufficient permissions, then the completion record reports the IDPT handle associated with the faulting address or error.

If an inter-domain operation is partially completed due to a page fault in an alternate PASID address space and the Window Mode field in the IDPTE is set to Offset Mode, the completion record does not report the fault address in the alternate address space, and the Fault Address Masked bit in the Fault Info field is reported as 1. The Fault Info field indicates the operand that caused the fault (described in section 8.2.3), and software can use the Bytes Completed field to identify the location of the fault relative to the starting address or offset specified in the descriptor.

When an inter-domain operation encounters a page fault in an alternate PASID address space, the submitter is expected to use software mechanisms to resolve it prior to resuming the operation (for example, by communicating with the owner process whose memory window experienced the page fault). Alternately the submitter can set the Block on Fault flag in the descriptor to cause the device to wait for the page fault to be resolved before continuing with the operation.

# 3.14.5 Memory Window Modification

For a SASS or SAMS IDPTE, if the Allow Update bit in the IDPTE is 1, the owner may modify the memory window attributes using an Update Window descriptor (section 8.3.27). Only the process whose PASID matches the access PASID in the IDPTE is allowed to issue the Update Window. If the descriptor PASID does not match the access PASID, the Update Window descriptor is completed with an error.

If Allow Update is 0 for an IDPTE, the entry may be modified by the kernel-mode driver using MMIO writes while the IDPTE is not usable. See section 9.2.29 for details on when different fields in an IDPTE may be modified.

An Update Window descriptor atomically changes only the values of Window Base, Window Size, Window Mode, Read and Write permissions, and the Window Enable field in the IDPTE. Since the update is done atomically by hardware, any inter-domain descriptor referencing the IDPTE at the same time, is guaranteed to see either the old value or the new value of the window attributes. After the atomic update is done, an Update Window descriptor also performs an implicit drain to flush out any in-flight descriptors that are still using pre-update window attributes of that IDPTE. This ensures that when an Update Window operation is completed, any prior descriptors referencing that IDPTE have also completed. As described in section 8.3.27, an Update Window descriptor also allows for the implicit drain to be suppressed, if necessary.

# 3.15 Administrative Commands

Administrative commands are submitted to the device by writing to the Command register. Administrative commands are used to enable and disable the device, enable and disable WQs, and drain and abort descriptors.

Only one command may be submitted at a time. Software must wait for a prior command to complete before submitting another command. To determine when a command has completed, software may poll

the Command Status register or request an interrupt by setting the Request Completion Interrupt field to 1 when it issues the command.

To ensure system reliability and availability and implement robust error handling, it is recommended that software implement mechanisms to handle cases where execution of an administrative command exceeds a software defined maximum time.

If Command Capabilities Support in GENCAP is 1, the Command Capabilities register (described in section 9.2.14) indicates which of the administrative commands are supported.

See the description of the Command register in section 9.2.12 for details on how to submit these commands. See the description of the Command Capabilities register in section 9.2.14 for information about which administrative commands are supported.

<table><tr><td>Enable Device</td><td>Check the device configuration and enable the device. The device must be enabled before enabling any WQs.</td></tr><tr><td>Disable Device</td><td>Stop accepting descriptors to all WQs, wait for completion of all descriptors, disable all WQs, and disable the device.</td></tr><tr><td>Enable WQ</td><td>Check the WQ configuration and enable the WQ. Once the command has successfully completed, descriptors may be submitted to the WQ.</td></tr><tr><td>Disable WQ</td><td>Stop accepting descriptors to the specified WQs, wait for completion of all descriptors that had been queued to the WQs, and disable the WQs.</td></tr><tr><td>Drain All</td><td>Wait for all descriptors in all WQs and all engines that were submitted prior to the Drain All command. The device may start work on new descriptors while the command is waiting for prior descriptors to complete; thus, descriptors submitted after the command may be in progress at the time the command completes.</td></tr><tr><td>Abort All</td><td>Abandon and/or wait for all descriptors in all WQs and all engines that were submitted prior to the Abort All command. Software must ensure that no descriptors are submitted to any WQs after the command is submitted and before it completes; otherwise, the behavior is undefined.</td></tr><tr><td>Drain WQ</td><td rowspan="2">Wait for all descriptors submitted to the specified WQs. Software must ensure that no descriptors are submitted to any of the specified WQs after the command is submitted and before it completes; otherwise, the behavior is undefined. Abort WQ may abandon some or all descriptors in the WQ instead of completing them.</td></tr><tr><td>Abort WQ</td></tr><tr><td>Drain PASID</td><td rowspan="2">Wait for all descriptors associated with the specified PASID in all WQs and all engines. When the command completes, there are no more descriptors for the PASID in the device. Software must ensure that no descriptors with the specified PASID are submitted to the device after the command is submitted and before it completes; otherwise, the behavior is undefined. Abort PASID may abandon some or all of the descriptors instead of completing them.</td></tr><tr><td>Abort PASID</td></tr><tr><td>Reset Device</td><td>Stop accepting descriptors on all WQs, abort all descriptors in the device, wait for any operations in flight, disable all WQs, disable the device, and clear the</td></tr><tr><td></td><td>entire device configuration to power-on values, except for the Command, Command Status, Software Error, and Event Log Status registers, and the MSI-X table. If the device is already disabled, only clear the device configuration. See Table 9-3 for the initial values of device registers.</td></tr><tr><td>Reset WQ</td><td>Stop accepting descriptors to the specified WQs, abort all descriptors in the WQs, wait for any operations in flight, and disable the WQs. Then reset the WQ configuration registers of the specified WQs to initial values, except the WQ Size fields that are not modified. For any of the specified WQs that are already disabled, only reset the WQ configuration registers. This command allows specification of multiple work queues. See Table 9-3 for the initial values of device registers.</td></tr><tr><td>Request Interrupt Handle</td><td>Request an interrupt handle that can be used in descriptors to request completion interrupts. If this command is supported (as indicated by the Command Capabilities register), software must use this command to obtain interrupt handles. The result of this command may be an error if no additional interrupt handles are available. See section 3.7 for more information.</td></tr><tr><td>Release Interrupt Handle</td><td>Release an interrupt handle previously returned by the Request Interrupt Handle command. This command may be used to free a handle that is no longer needed. The released handle may not be used to request interrupts once this command has been issued. If any previously submitted descriptors using the released handle have not yet completed, the behavior is undefined.</td></tr><tr><td>Request IDPT Handle</td><td>Request a handle that can be used in descriptors to reference the specified IDPT entry in the Inter-Domain Permissions Table. If the Request IDPT Handle field in CMDCAP is 1, software must use this command to obtain IDPT handles. If the Request IDPT Handle field in CMDCAP is 0, this command code is reserved, and software uses the index of the entry in the Inter-Domain Permissions Table as the handle.</td></tr><tr><td>Release IDPT Handle</td><td>Release an IDPT handle previously returned by the Request IDPT Handle command. Software may use this command to release a handle for an entry in the Inter-Domain Permissions Table that is no longer in use. When the Request IDPT Handle command is supported, hardware may revoke an IDPT handle at any time. Software should use this command to release the expired handle after acquiring a new handle for the IDPT entry. See section 7.5.6 for more information.</td></tr><tr><td>Invalidate Submitter Bitmap Cache</td><td>Invalidate all or a portion of the submitter bitmap cache. If the Invalidate Submitter Bitmap Cache field in CMDCAP is 1, software must issue this command after updating any part of a submitter bitmap or modifying the address mapping of any page of the bitmap. If the Invalidate Submitter Bitmap Cache field in CMDCAP is 0, this command code is reserved, and no bitmap cache invalidation is required.</td></tr></table>

# Drain and Abort Commands

Upon completion of any command that waits for or abandons descriptors, hardware guarantees that no further memory writes or interrupts will be generated due to any of the affected descriptors. Depending on the implementation, any drain command may wait for completion of other descriptors in addition to the descriptors that it is required to wait for.

When any type of abort command is issued, the hardware may either abandon or complete any of the affected descriptors. Some descriptors may be completed while others are abandoned. If a descriptor is completed, all the associated memory accesses, completion record, and completion interrupt are performed. If a descriptor is abandoned, no completion record is written and no completion interrupt is generated for that descriptor, but some or all of the other memory accesses may occur. Since the abort and reset commands are not guaranteed to abandon operations that have already started, they are not effective to terminate operations that are taking longer than expected. (Software may use FLR to do this if it is necessary.) The maximum size of operations may be limited using the WQ Maximum Transfer Size and WQ Maximum Batch Size configuration registers.

# Software Usage of Drain and Abort Commands

When an application or VM that is using Intel DSA is suspended, it may have outstanding descriptors submitted to the device. This work must be completed so the client is in a coherent state that can be resumed later. The Drain PASID and Drain All commands are used by the OS or VMM to wait for any outstanding descriptors. The Drain PASID command is used for an application or a VM that was using a single PASID. The Drain All command is used for a VM using multiple PASIDs.

When an application that is using the device exits or is terminated by the OS, the OS needs to ensure that there are no outstanding descriptors before it can free up or re-use address space, allocated memory, and the PASID. To clear out any outstanding descriptors, the OS uses the Abort PASID command with the PASID of the client being killed. On receiving this command, the device discards all descriptors belonging to the specified PASID without further processing.

# 3.16 Virtualization

The Intel DSA architecture is designed to be easy and efficient to virtualize. Intel DSA supports the Intel Scalable I/O Virtualization model. For more details on the Intel Scalable IOV architecture, refer to the Intel® Scalable I/O Virtualization Technical Specification, listed in the References in section 1.2.

Intel DSA has the following features designed to support efficient virtualization. The design of software to use these features is described in section 7.3.

- Directly accessible MMIO registers: MMIO space lays out performance critical registers (i.e., portals) in separate 4K pages to allow direct mapping to VMs using CPU Extended Page Tables (EPT).

- Minimize client specific state: The architecture has been designed to store minimal client specific state on the device to increase scalability. For example, the descriptors have been designed so that the information required to process the descriptors is included in the descriptors themselves.

- Capabilities: Software reads capability registers to detect support for features such as block-on-fault. Through capability virtualization, the VMM can expose a subset of the device's capabilities to

VMs, which helps in VM image deployment and VM migration across multiple generations of Intel DSA devices with different capabilities.

- Intel Scalable IOV: The Intel Scalable IO Virtualization architecture reduces virtualization complexity and allows the device to be shared across a large number of VMs.

Guest OS interrupts: Intel DSA allows guests to request descriptor completion interrupts that are delivered directly to the VM. A guest uses the Request Interrupt Handle to request Completion Interrupt Handles to use in its descriptors to request completion interrupts. Each handle denotes an entry in the Interrupt Message Storage that has been configured as an interrupt for that guest. The device uses the IMS entry to send interrupts to the VM.

- Guest IDPT: Intel DSA defines a command for a guest to request IDPT Handles to use in its descriptors to perform inter-domain operations. Each handle denotes an entry in the IDPT that is allocated to that guest. There is also a command to invalidate the IDPT bitmap cache, which can be used by a VMM to shadow guest changes to the submitter bitmaps.

S

# 4 Quality of Service Control

# 4.1 Work Dispatch Priority

Intel DSA provides WQ priorities to control quality of service for dispatching work from multiple WQs in the same group. The priority of each WQ is specified in its WQ Configuration register, described in 9.2.24. WQ priority levels range from 1 to 15. The WQ priority is relative to other WQs in the same group. Work queues in a group may have the same or different priorities.

The arbiter for each group dispatches descriptors from the WQs in the group according to their priority using the following procedure: Each WQ has a counter that is initialized to the WQ's priority level and decremented each time a descriptor is dispatched from the WQ. The arbiter for each group iterates through the WQs in the group, dispatching one descriptor from each WQ that has a descriptor available and has a non-zero counter. Once the counter for a WQ reaches zero, no more descriptors are dispatched from that WQ until the counter is reset. Once all counters reach zero for all WQs in a group that have pending descriptors, the counters for all WQs in the group are reinitialized to the respective WQ's priority level.

Thus, for example, a WQ with a priority of 6 will issue 3 times as many descriptors to the engine as a WQ with a priority of 2 (assuming that both WQs have descriptors available at all times).

When software submits a descriptor to a WQ that was previously empty, the descriptor will be processed at that WQ's next turn, regardless of the WQ's priority level, if the WQ's counter is non-zero.

There is no delay caused by the arbiter checking empty WQs to see if they have descriptors available. Descriptors can be issued to the engines in the group at the rate the engines can process them, even if the only WQ(s) with descriptors available have low priority.

# 4.2 Traffic Classes

Intel DSA includes support for Traffic Classes as defined in PCIe, if the PCIe VC capability is present. Traffic Classes may be used by the platform outside of the device to control QoS for memory transactions initiated by the device.

Traffic Classes are also used within the device to segregate traffic destined for low bandwidth memory. Each platform has one or more designated traffic class values that should be used for accesses to low bandwidth memory. See section 4.5 for information on configuring traffic classes for use with low bandwidth memory.

There are two traffic class registers in each Group Configuration register. Traffic Classes specified in the Group Configuration traffic class registers must have valid mappings in the TC/VC Maps in the PCIe Virtual Channel Extended Capability structure. Each descriptor has flags to select which of the two traffic classes to use for each address used by the descriptor. For best results, software should ensure that operations with dissimilar QoS characteristics are issued to different groups.

# 4.3 Read Buffer Allocation

The Intel DSA device uses read buffers to hide memory read latency. Software can control how these read buffers are allocated, which affects the read bandwidth available to certain guests or applications.

Read Buffers are resources within the Intel DSA implementation that are allocated to engines to support memory read operations. The total number of Read Buffers supported is fixed by the implementation and is reported in the GRPCAP register. Limiting the number of Read Buffers available to a group can restrict the read bandwidth usable by engines in the group. The relationship between Read Buffers and actual bandwidth is dependent on instantaneous system memory latency and varies dynamically as system utilization changes. Read Buffers are internal to the design of Intel DSA and are not related to other resources in the SoC that also affect the bandwidth available to the device.

The policy by which Read Buffers are allocated to groups is based on two fields in the Group Configuration registers. The Read Buffers Reserved field indicates the number of Read Buffers set aside for the exclusive use of engines in the group. The Read Buffers Allowed field indicates the maximum number of Read Buffers that may be in use at one time by all engines in the group. (Read Buffers allocated to a group may also be limited by the Global Read Buffer Limit, as described in section 9.2.8.)

Setting the Read Buffers Reserved field to a non-zero value ensures that engines in the group are able to acquire Read Buffers without being starved by engines in other groups. However, reserving Read Buffers for a group limits the number of Read Buffers available to other groups, potentially limiting their ability to efficiently utilize the bandwidth capability of the device. The sum of the Read Buffers reserved for all groups must be no greater than the total number of Read Buffers available (as reported in GRPCAP).

For each group, the Read Buffers Allowed field must be greater than or equal to 4 times the number of engines in the group. It must also be no greater than the value of the Read Buffers Reserved field for that group plus the number of non-reserved Read Buffers. The number of non-reserved Read Buffers is defined as the total number of Read Buffers supported minus the number of Read Buffers reserved for all groups combined. For example, if the device supports 74 Read Buffers and 3 are reserved for each of the four groups, then 62 Read Buffers remain non-reserved, and the maximum value of Read Buffers Allowed for each group would be 65.

There is a system-specific value for the Read Buffers Allowed field (dependent on the read latency of the system) that allows the group to utilize the full bandwidth of the device. This value can be calibrated by software. There is no advantage to setting the Read Buffers Allowed field to a greater value. Setting this field to a smaller value limits the Read Buffers that can be allocated to engines in the group, thereby limiting their impact on the performance of engines in other groups.

# 4.4 Latency Control

Intel DSA provides controls for software to reduce the latency impact of multiple outstanding descriptors being in progress in the device. Within an engine, multiple descriptors may be processed concurrently, in a pipelined manner. If the Descriptors in Progress Limit Supported field in GRPCAP is 1, the Maximum Work Descriptors in Progress and Maximum Batch Descriptors in Progress fields in ENGCAP specify the maximum number that may be concurrently in process in an engine at a given time. Software can use the Work Descriptors in Progress Limit and Batch Descriptors in Progress Limit fields

in GRPCFG to limit it to a value smaller than the maximum supported by the implementation. The GRPFLAGS register allows software to configure the Work Descriptors in Progress Limit and Batch Descriptors in Progress Limit fields as a fraction of the corresponding maximum permissible value. The values specified apply to each engine in the group. Specifying a value smaller than the maximum can sometimes result in a lower achievable bandwidth depending on system latency conditions. Software can set the limit fields judiciously to achieve the right balance between peak performance and quality of service considerations.

The arbiter for each group dispatches descriptors from a WQ to an engine in the group up to the maximum value configured for the engine. Once the number of descriptors dispatched to an engine reaches the maximum value, further dispatch to the engine is stalled until one or more descriptors are completed. During this time, additional descriptors may queue up in the WQ and eventually result in back pressure to software attempting to submit additional descriptors. Software can use this control along with the WQ size, maximum transfer size, and maximum batch size controls in WQ Configuration to limit the maximum latency that a descriptor may experience due to prior descriptors. The actual realizable latency benefit is implementation and system dependent. Depending on prevailing system latency values, use of this control could result in lower effective memory bandwidth from an engine.

# 4.5 Low Bandwidth Memory

Intel DSA includes features to improve system performance when accessing memory with lower bandwidth or higher latency than main memory, such as CXL-attached memory. When Intel DSA is used to read and/or write to these types of memory, software should take these steps to limit the impact to the throughput of other operations, both within the device and throughout the platform.

1. Set the Global Read Buffer Limit field in GENCFG to a suitable value for the bandwidth available. (This value is platform dependent and can be calibrated by software.)

2. Create one or more groups that will be used with descriptors that access low bandwidth memory.

3. Set the Use Global Read Buffer Limit field to 1 in the Group Configuration register for those groups.

4. Set TC-B field in those groups to a Traffic Class value that is designated for use with low bandwidth memory.

5. Each descriptor should set the TC Selector flags to indicate which of its source and destination addresses refer to low bandwidth memory.

Software must take care to submit work to a suitable group and to correctly classify each buffer address in a descriptor and set the TC Selector flags. If a descriptor referencing low bandwidth memory is submitted to a group that is not configured to support low bandwidth memory, or if a TC Selector flag in a descriptor incorrectly indicates that the corresponding address is not in low bandwidth memory, the memory transaction may be blocked by the platform. If the memory transaction is a write operation, software will not be notified.

If software cannot correctly classify its buffers, for example, if the memory allocation strategy of system software mixes normal and low bandwidth memory in such a way that an application cannot tell which type of memory it has received, then both the TC-A and TC-B fields of GRPCFG should be set to TC values that are suitable for low bandwidth memory.

The number of Read Buffers specified by Global Read Buffer Limit is shared by all descriptors executing in all groups for which Use Global Read Buffer Limit is 1. The engine executing the descriptor is also limited by the Read Buffers Allowed field in GRPCFG.

# 4.6 Bandwidth Control

Intel DSA provides controls for software to limit the maximum read and write bandwidth per group. If Bandwidth Limit Support in GRPCAP is 1, software can use the Read Bandwidth Limit and Write Bandwidth Limit fields in GRPFLAGS to limit the maximum read and write bandwidth for that group. The limits are expressed as a fraction of the maximum value supported by the device implementation. Hardware throttles an engine when the bandwidth utilization of the group equals or exceeds the configured maximum. As a result, the instantaneous bandwidth of the group may be higher or lower than the configured limit; but it can never exceed the maximum value supported by the device implementation.

S

# 5 Error Handling

The primary goals of Intel DSA error detection and handling are:

- Avoid writing incorrect data or writing to an incorrect address.

- Avoid errors due to one client from affecting work for other clients.

- Avoid misinterpreting an erroneous operation descriptor.

- Report errors to the client that submitted the work when possible.

- Provide enough information to continue an operation that was partially completed.

- Provide enough information to help diagnose the cause of the error.

Errors associated with the processing of a descriptor are reported in the completion record of the descriptor (if the completion record address is valid).

Hardware errors are reported via PCI Express Advanced Error Reporting. Hardware errors include errors in the fabric and errors internal to the device. If a hardware error is associated with a descriptor and the Completion Record Address in the descriptor is valid, the error is also reported in the completion record.

Errors on the completion record of a descriptor or during processing of a descriptor that does not have a valid completion record address are reported in the Software Error Register or event log. A synopsis of software error handling is shown in Table 5-1.

# 5.1 Device Enable Checks

The device performs the following checks at the time the Enable Device command is issued to the Command Register:

- Bus Master Enable is 1.

The sum of the WQ Size fields of all the WQCFG registers is not greater than Total WQ Size.

- For each GRPCFG register:

The WQs and Engines fields are either both zero or both non-zero.

Bits in the WQs field beyond the number of WQs are 0.

Bits in the Engines field beyond the number of Engines are 0.

o Reserved bits in the Flags field are 0.

For Group Configuration registers beyond the number of groups, all fields are zero.

Each WQ for which the Size field in the WQCFG register is non-zero is in exactly one group.

Each WQ for which the Size field in the WQCFG register is zero is not in any group.

Each engine is in no more than one group.

- If the Global Read Buffer Limit Supported field in GRPCAP is 0, then the Use Global Read Buffer Limit field is 0 in every GRPCFG register.

- If the Bandwidth Limit Support field in GRPCAP is 1, the Read Bandwidth Limit and Write Bandwidth Limit fields in each GRPCFG register have valid values. If Bandwidth Limit Support is 0, these fields must be 0.

- If the Global Read Buffer Limit Supported field in GRPCAP is 1, then the Global Read Buffer Limit in GENCFG is less than or equal to the Total Read Buffers field in GRPCAP.

- If the Use Global Read Buffer Limit field is 1 in any GRPCFG register, then the Global Read Buffer Limit in GENCFG is at least 4 times the total number of engines in all groups that have the Use Global Read Buffer Limit set to 1.

- If the Read Buffer Controls Supported field in GRPCAP is 1, then the sum of the Read Buffers Reserved fields, for all groups that have engines assigned, are less than or equal to the Total Read Buffers field in GRPCAP.

- If the Read Buffer Controls Supported field in GRPCAP is 1, then for each group that has engines assigned to it, Read Buffers Allowed is:

o Greater than or equal to 4 times the number of engines in the group;

o Greater than or equal to the Read Buffers Reserved field for the group; and

Less than or equal to the sum of the Read Buffers Reserved field and the number of non-reserved Read Buffers.

- If the Enable bit in PRSCTL is 1, then the number of Outstanding Page Requests Allowed in PRSREQALLOC is non-zero and is less than or equal to the maximum number of Page Requests supported in PRSREQCAP.

- If Event Log Enable in GENCFG is 1:


Event Log Support in GENCAP is not 0.


<table><tr><td>Category</td><td>Error Type</td><td>Intel® DSA Handling</td></tr><tr><td rowspan="2">Descriptor submission</td><td>Posted write to SWQ. 
Posted write to WQ that is not Enabled. 
Non-64-byte write to any portal.</td><td>Ignored.</td></tr><tr><td>DMWr to DWQ. 
DMWr to WQ that is not Enabled. 
DMWr to non-WQ address.</td><td>Returns Retry.</td></tr><tr><td rowspan="5">Descriptor errors</td><td>Misaligned completion record address.</td><td rowspan="2">Reported in SWERROR or event log.</td></tr><tr><td>Failure translating completion record address.</td></tr><tr><td>Descriptor decode error: invalid operation, invalid flags, non-zero reserved field, etc.</td><td rowspan="2">Completion Record Address Valid = 1: 
Reported in completion record. 
Completion Record Address Valid = 0: 
Reported in SWERROR register or event log.</td></tr><tr><td>Error in descriptor processing (e.g., PRS failure, IDPT permission violation).</td></tr><tr><td>Invalid interrupt handle.</td><td>Completion Record Address Valid = 1: 
Reported in completion record and in SWERROR¹ or event log. 
Completion Record Address Valid = 0: 
Reported in SWERROR register or event log.</td></tr><tr><td rowspan="3">Configuration errors</td><td>Invalid device configuration when Enable Device command is issued.</td><td>Reported in Command Status register. Device is not enabled.</td></tr><tr><td>Invalid work queue configuration when WQ Enable command is issued.</td><td>Reported in Command Status register. WQ is not enabled.</td></tr><tr><td>Unsupported change to PCI configuration while device is not Disabled (including BME, ATS, PASID, and PRS).</td><td>Device enters Halt state. Reported in SWERROR register.</td></tr></table>

Table 5-1: Handling of Software Errors

Event Log Size  $\geq 64$

Event Log Base Address + Event Log Size × Event Log entry size ≤ 2 $^{64}$ .

- In cases where the platform has indicated a requirement to either use ATS or not to use ATS, the setting of ATS Enable matches the requirement.

- If the PASID Enable field in EVLCFG is 1, then PASID Enable in the PCI Express PASID capability is 1.

- If the Priv field in EVCLFG is 1, then PASID Enable in EVLCFG and Privileged Mode Enable in the PCI Express PASID capability are both 1.

- If the PASID Enable field in IDBR is 1, then PASID Enable in the PCI Express PASID capability is 1

- If the Priv field in IDBR is 1, then PASID Enable in IDBR and Privileged Mode Enable in the PCI Express PASID capability are both 1.

If any of the device enable checks fail, the device is not enabled and the error is reported in the Command Status register. These checks may be performed in any order. Thus, an indication of one type of error does not imply that there are not also other errors. The same configuration errors may result in different error codes at different times or with different versions of the device.

If none of the checks fail, the device is enabled and the Command Status register is set to indicate successful completion of the Enable Device command. If Event Log Enable in GENCFG is 1, the Event Log Status register is set to 0.

# 5.2 WQ Enable Checks

The device performs the following checks at the time the Enable WQ command is issued to the Command Register:

The device is Enabled.

- The WQ parameter is less than the number of work queues.

The WQ is Disabled.

- The WQ Size field is non-zero.

- The WQ Mode field selects a supported mode. That is, if the Shared Mode Support field in WQCAP is 0, WQ Mode is 1; or if the Dedicated Mode Support field in WQCAP is 0, WQ Mode is 0. If both the Shared Mode Support and Dedicated Mode Support fields are 1, either value of WQ Mode is allowed.

- If WQ Priority Support is 1, the WQ Priority field is non-zero.

- If the Block on Fault Support field in GENCAP is 0 or the Enable field of the PCIe Page Request Control register is 0, the WQ Block on Fault Enable field is 0.

- If the WQ Mode field is 0, the WQ PASID Enable field is 1.

- If the PASID Enable field of the PCI Express PASID capability is 0, the WQ PASID Enable field is 0. (This rule, in combination with the above rule, means that Shared WQs cannot be used when the PASID capability is disabled.)

- If the WQ Mode field is 1, WQ PASID Enable is 1, and the Privileged Mode Enable field of the PCI Express PASID capability is 0, then the WQ Priv field is 0.

- The WQ Maximum Transfer Size field is not greater than the Maximum Supported Transfer Size field in GENCAP.

- The WQ Maximum Batch Size field is greater than 0 and not greater than the Maximum Supported Batch Size field in GENCAP.

- If the Interrupt Message Storage Size field in GENCAP is 0, WQ Occupancy Interrupt Table is 0.

- If the Request Interrupt Handle command is not supported, then WQ Occupancy Interrupt Handle is less than the size of the selected interrupt table (the MSI-X table if WQ Occupancy Interrupt Table is 0; the IMS table if WQ Occupancy Interrupt Table is 1).

- If the Request Interrupt Handle command is supported, then WQ Occupancy Interrupt Handle is a handle returned by that command.

- If WQ Operations Configuration Support in WQCAP is 1, the OPCFG field in WQCFG does not have any bits set to 1 that are not set to 1 in OPCAP. That is, WQOPCFG AND ~OPCAP is 0.

- If WQ ATS Support in WOCAP is 0, WQ ATS Disable is 0.

- If WQ PRS Support in WQCAP is 0 or Event Log Enable in GENCFG is 0 or WQ Block on Fault Enable is 1, then WQ PRS Disable is 0.

If any of the WQ enable checks fail, the WQ is not enabled and the error is reported in the Command Status register. These checks may be performed in any order. Thus, an indication of one type of error does not imply that there are not also other errors. The same configuration errors may result in different error codes at different times or with different versions of the device.

If none of the checks fail, the WQ is enabled and the Command Status register is set to indicate successful completion of the Enable WQ command.

# 5.3 Descriptor Submission Checks

The device performs the following checks in order when a descriptor is received. Except as noted, if any of these checks fail, the descriptor is discarded, and if the descriptor was submitted with DMWr, a Retry response is returned.

The WO identified by the portal address used to submit the descriptor is Enabled.

If the descriptor was submitted to a shared WO:

o . It was submitted with DMWr (e.g., using the ENOCMD or ENOCMDS instruction).

If the descriptor was submitted via a limited portal, the current queue occupancy is less than the WQ Threshold.2

If the descriptor was submitted via an unlimited portal, the current queue occupancy is less than WO Size.

If the descriptor was submitted to a dedicated WQ:

o It was submitted with a posted, aligned 64-byte write (e.g., using the MOVDIR64B instruction).

The queue occupancy is less than WQ Size. If this check fails, the descriptor is discarded, and the error is recorded in SWERROR.

Note that if MOVDIR64B is used to write to a disabled WQ, a shared WQ, or an invalid portal address, the write is discarded without notification to software.

# 5.4 Descriptor Checks

The device performs the following checks on each descriptor when it is processed:

- If the Completion Record Address Valid flag is 1, the Completion Record Address is 32-byte aligned.

- The value in the operation code field corresponds to a supported operation, as specified in WQ OPCFG, if WQ Operations Configuration Support in WQCAP is 1; or as indicated by OPCAP, if WO Operations Configuration Support in WOCAP is 0.

- The operation is valid in the context in which it was submitted. Batch, Drain, and Update Window operations are not supported inside a batch and are treated as invalid operation codes. A Translation Fetch operation is invalid when ATS is disabled or when submitted to a WQ with WQ ATS Disable set to 1 in the WQ configuration.

- No reserved flags are set. This includes flags for which the corresponding capability bit in the GENCAP register is 0.

- No unsupported flags in the Flags field are set. This includes flags that are reserved for use with certain operations. For example, the Fence bit is reserved in descriptors that are not part of a batch. It also includes flags that are disabled in the configuration, such as the Block On Fault flag, which is reserved when the Block On Fault Enable field in the WQCFG register is 0. See Table 5-3 and Table 5-4 for details.

- Required flags in the Flags field are set. For example, the Request Completion Record flag must be 1 in a descriptor for the Compare operation. See Table 5-5 for details.

- Reserved fields (other than flags) are 0. This includes any fields that have no defined meaning for the specified operation. Some implementations may not check all reserved fields, but software should take care to clear all unused fields for maximum compatibility.

- In a descriptor submitted to a shared WQ, if the Privileged Mode Enable field of the PCI Express PASID capability is 0, the Priv field is 0.

- The Transfer Size (if applicable for the descriptor type) is greater than 0 and not greater than the value specified by the WQ Maximum Transfer Size field in the WQ Config register.

- Destination buffers do not overlap source buffers or other destination buffers. However, this check does not apply to:

Memory Move operation if the Overlapping Copy Support capability is 1.

Type Conversion, Reduce, or Reduce with Dualcast operations if the source and destination buffer addresses are equal and IData Type and OData Type are the same.

o Descriptors that specify an SGL.

<table><tr><td>Submission Portal</td><td>Request Interrupt Handle Command Available</td><td>Completion Interrupt Handle Check</td></tr><tr><td>MSI-X</td><td>No</td><td>Less than the size of the MSI-X table.</td></tr><tr><td>IMS</td><td>No</td><td>Less than Interrupt Message Storage Size.</td></tr><tr><td>MSI-X</td><td>Yes</td><td>A valid MSI-X handle returned by the Request Interrupt Handle command that has not been revoked.</td></tr><tr><td>IMS</td><td>Yes</td><td>A valid IMS handle returned by the Request Interrupt Handle command that has not been revoked.</td></tr></table>

Table 5-2: Completion Interrupt Handle Checks

If the Request Completion Interrupt flag is 1, the Completion Interrupt Handle is valid according to Table 5-2.

- If the Request Completion Interrupt flag is 1, the PASID Enable field in the selected interrupt table entry equals the WQ PASID Enable control for the work queue the descriptor was submitted to. Furthermore, if the PASID Enable field is 1, the PASID field in the selected interrupt table entry equals the PASID of the descriptor.

- The Traffic Classes selected by descriptor flags have the corresponding bits set in the TC/VC map of a VC Resource Control register, and the VC Enable bit in that register is 1.

In a Batch descriptor:

If Batch1 Support in GENCAP is 0, the Descriptor Count field is greater than 1.

o If Batch1 Support is 1, Descriptor Count is non-zero.

o Descriptor Count is not greater than the value specified by the WQ Maximum Batch Size field in the WQ Config register.

- In a Create Delta Record or Apply Delta Record descriptor, the Transfer Size is not greater than the allowed value (0x80000 bytes, or 512 KB, as described in section 8.3.8).

- In a Create Delta Record or Apply Delta Record descriptor, the Maximum Delta Record Size or Delta Record Size (as applicable for the descriptor type) is not greater than the value specified by the WQ Maximum Transfer Size field in the WQ Config register.

- In a Create Delta Record descriptor, the Maximum Delta Record Size is greater than or equal to 80 bytes.

In an Apply Delta Record descriptor, the Delta Record Size is greater than or equal to 10 bytes.

- In a Memory Copy with Dualcast or Reduce with Dualcast descriptor, bits 11:0 of the two destination addresses are the same.

- In a CRC Generation or Copy with CRC Generation descriptor, if the Read CRC Seed flag is 1, CRC Seed Address is aligned to the size of the CRC.

- In a Translation Fetch descriptor, the Region Size is non-zero, and the sum of Address and Region Size is less than or equal to  $2^{64}$ .

- In a Translation Fetch descriptor, the Region Stride is greater than or equal to 4096 and is a power of 2.

- In any descriptor that specifies a data type, the data type is one of the supported types in the Data Types Supported field in DSACAP1.

- In a Type Conversion, Reduce, or Reduce with Dualcast descriptor, the transfer size indicated by the product of Element Count and the size of either data type is not greater than the WQ Maximum Transfer Size field in WQCFG.

- In a Reduce or Reduce with Dualcast descriptor, if the Use Inter-Domain Selector flag is 1, then Inter-Domain Selector field is non-zero and has a valid value as described in section 8.1.13.

- In operations that specify an SGL in the descriptor:

The SGL Format field specifies a supported format, as indicated by the SGL Formats Supported field in DSACAPO.

o SGL Size is non-zero and not greater than the WQ Maximum SGL Size in WQCFG.

In a Gather Reduce descriptor:

The block size is not greater than Maximum Supported Gather Reduce Block Size in DSACAPO.

The total transfer size indicated by the product of block size and SGL Size is not greater than WQ Maximum Transfer Size in WQCFG.

- In a Reduce, Reduce with Dualcast, or Gather Reduce descriptor, Compute Type is one of the supported operations in the Compute Operations Supported field in DSACAP1.

In a Type Conversion, Reduce, Reduce with Dualcast, or Gather Reduce descriptor:

○ All source addresses are aligned to the size of the input data type.

o All destination addresses are aligned to the size of the output data type.

○ IData Type and OData Type are either both integer type or both floating-point type.

o If IData Type and OData Type are floating-point types and they are not the same, then the conversion from IData Type to OData Type is supported, as indicated by the Floating-Point Conversion Support fields in DSACAP1.

- Any flags set in Compute Flags are supported, as indicated by the corresponding fields in DSACAP2.

The Flush to Zero field in Compute Flags is zero if the output data type is not a floating-point type.

The Treat Denormal as Zero field in Compute Flags is zero if the input data type is not a floating-point type.

The Rounding Type field in Compute Flags is zero if neither data type is a floating-point type.

The Treat Integer Operands as Signed Values field in Compute Flags is zero if the input data type is not an integer type.

The Saturate Integer Result field in Compute Flags is zero if the output data type is not an integer type.

- In a Gather Copy, Scatter Copy or Scatter Fill descriptor, the Transfer Size is equal to the product of SGL Size, Element Count, and the size of the data type.

- In a Gather Reduce or Gather Copy descriptor, the destination buffer does not overlap the SGI.

- In an Inter-Domain Copy or Inter-Domain Compare descriptor, at least one of the Use Alternate PASID flags is 1.

In an Inter-Domain Memory Fill descriptor, the Use Alternate Destination PASID flag is 1.

In an Inter-Domain Compare Pattern descriptor, the Use Alternate Source PASID flag is 1.

- For an inter-domain or Update Window descriptor, the WQ PASID Enable control for the work queue the descriptor was submitted to is 1.

- If the Request IDPT Handle command is not supported, any IDPT handle in an inter-domain or Update Window descriptor is less than the Inter-Domain Permissions Table size.

- If the Request IDPT Handle command is supported, any IDPT handle in an inter-domain or Update Window descriptor is a handle returned by that command.

- In an Update Window descriptor, if the Window Enable flag is 1, the Window Size field is non-zero and Window Base plus Window Size is less than or equal to  $2^{64}$ .

If the Completion Record Address Valid flag is 0 and any of these checks fail, the error is reported in the Software Error register or event log.

If the Completion Record Address Valid flag is 1 and the Completion Record Address is misaligned or cannot be translated, or the completion record TC is invalid, then the descriptor is discarded and an error is reported in the Software Error Register or event log.

Otherwise, if any of these checks fail, the completion record is written with the Status field indicating the type of check that failed and Bytes Completed set to 0. If one of the flags checks fails, the Invalid Flags field of the completion record indicates flags that are invalid. A completion interrupt is generated, if

requested, unless a check related to completion interrupt delivery failed. If an invalid interrupt handle was specified, the error is also reported in the Event Log, if enabled, or in the Software Error register.<sup>1</sup>

These checks may be performed in any order. Thus, an indication of one type of error in the completion record does not imply that there are not also other errors. The same invalid descriptor may report different error codes at different times or with different versions of the device.

# 5.5 Descriptor Reserved Field Checking

Reserved fields in descriptors fall into three categories: fields that are always reserved; fields that are reserved under some conditions (e.g., based on a capability, configuration field, how the descriptor was submitted, or values of other fields in the descriptor itself); and fields that are reserved based on the operation type. For additional details on descriptor formats, see chapter 8.

Table 5-3 lists the flags and fields that are allowed and reserved for each operation type. Flags not listed are allowed for all operation types. Flag bits 5, 6, and 15 are reserved for all operation types. Flag bits 23:16 are operation specific and are reserved except when the operation description describes their use. Table 5-4 lists additional conditions under which certain flags and fields are reserved. Additional operation-specific reserved fields and flags are described with the respective descriptor details in section 8.3. Table 5-5 lists the operation types that require certain flags to be set to 1.

<table><tr><td rowspan="2">Operation</td><td colspan="7">Allowed Flags</td><td rowspan="2">Reserved Fields</td></tr><tr><td>Block on Fault</td><td>Check Result</td><td>Cache control flags</td><td>Strict Ordering</td><td>Address 1 TC Selector</td><td>Address 2 TC Selector</td><td>Address 3 TC Selector</td></tr><tr><td>No-op</td><td></td><td></td><td></td><td></td><td></td><td></td><td></td><td>Bytes 16-35; 38-63</td></tr><tr><td>Batch</td><td></td><td></td><td></td><td></td><td>■</td><td></td><td></td><td>Bytes 24-31; 38-63</td></tr><tr><td>Drain</td><td></td><td></td><td></td><td></td><td>■</td><td>■</td><td></td><td>Bytes 32-35; 38-63</td></tr><tr><td>Memory Move</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td></td><td>Bytes 38-63</td></tr><tr><td>Fill</td><td>■</td><td></td><td>■</td><td>■</td><td></td><td>■</td><td></td><td>Bytes 38-63</td></tr><tr><td>Compare</td><td>■</td><td>■</td><td></td><td></td><td>■</td><td>■</td><td></td><td>Bytes 38-39; 41-63</td></tr><tr><td>Compare Pattern</td><td>■</td><td>■</td><td></td><td></td><td>■</td><td></td><td></td><td>Bytes 38-39; 41-63</td></tr><tr><td>Create Delta Record</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>Bytes 38-39; 52-55; 57-63</td></tr></table>


<sup>1</sup> In some implementations, an invalid interrupt handle error that is reported in the completion record is not also reported in SWERROR.


<table><tr><td rowspan="2">Operation</td><td colspan="7">Allowed Flags</td><td rowspan="2">Reserved Fields</td></tr><tr><td>Blockon Fault</td><td>Check Result</td><td>Cache control flags</td><td>Strict Ordering</td><td>Address 1 TC Selector</td><td>Address 2 TC Selector</td><td>Address 3 TC Selector</td></tr><tr><td>Apply Delta Record</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td></td><td>Bytes 38-39; 44-63</td></tr><tr><td>Memory Copy with Dualcast</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>Bytes 38-39; 48-63</td></tr><tr><td>Translation Fetch</td><td>■</td><td></td><td></td><td></td><td></td><td></td><td></td><td>Bytes 24-31; 38-47; 52-63</td></tr><tr><td>CRC Generation</td><td>■</td><td></td><td></td><td></td><td>■</td><td></td><td>■</td><td>Bytes 24-31; 38-39; 44-63</td></tr><tr><td>Copy with CRC Generation</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>Bytes 38-39; 44-63</td></tr><tr><td>DIF Check</td><td>■</td><td></td><td></td><td></td><td>■</td><td></td><td></td><td>Bytes 24-31; 38-39; 41; 43-47; 56-63</td></tr><tr><td>DIF Insert</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td></td><td>Bytes 38-39; 40; 43-55</td></tr><tr><td>DIF Strip</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td></td><td>Bytes 38-39; 41; 43-47; 56-63</td></tr><tr><td>DIF Update</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td></td><td>Bytes 38-39; 43-47</td></tr><tr><td>DIX Generate</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td></td><td>Bytes 38-39; 40; 43-55</td></tr><tr><td>Type Conversion</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td></td><td>Bytes 38-55; 57[3:0]; 59</td></tr><tr><td>Reduce</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>Bytes 38-39; 48-55</td></tr><tr><td>Reduce with Dualcast</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>Bytes 38-39</td></tr><tr><td>Gather Reduce</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>Bytes 38-43; 46-47; 59[3:0]</td></tr><tr><td>Gather Copy</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>Bytes 38-39; 46-47; 56[7:4]; 57-58; 59[3:0]</td></tr><tr><td>Scatter Copy</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td>■</td><td>Bytes 38-39; 46-47; 56[7:4]; 57-58; 59[3:0]</td></tr><tr><td>Scatter Fill</td><td>■</td><td></td><td>■</td><td>■</td><td></td><td>■</td><td>■</td><td>Bytes 38-39; 46-47; 56[7:4]; 57-58; 59[3:0]</td></tr><tr><td>Cache flush</td><td>■</td><td></td><td>1</td><td>■</td><td></td><td>■</td><td></td><td>Bytes 16-23; 38-63</td></tr><tr><td>Update Window</td><td></td><td></td><td></td><td></td><td></td><td></td><td></td><td>Bytes 32-35; 38-60</td></tr><tr><td>Inter-Domain Copy</td><td>■</td><td></td><td>■</td><td>■</td><td>■</td><td>■</td><td></td><td>Bytes 38-59</td></tr><tr><td>Inter-Domain Fill</td><td>■</td><td></td><td>■</td><td>■</td><td></td><td>■</td><td></td><td>Bytes 38-39; 48-61</td></tr><tr><td>Inter-Domain Compare</td><td>■</td><td>■</td><td></td><td></td><td>■</td><td>■</td><td></td><td>Bytes 38-39; 41-59</td></tr><tr><td>Inter-Domain Compare Pattern</td><td>■</td><td>■</td><td></td><td></td><td>■</td><td></td><td></td><td>Bytes 38-39; 41-59; 62-63</td></tr></table>


See the description of the Cache Flush operation (8.3.26) for the allowed flag combinations.



Table 5-3: Supported Flags and Reserved Fields by Operations


<table><tr><td>Reserved Field</td><td>Conditions Under Which Field is Reserved</td></tr><tr><td>Request Completion Interrupt</td><td>User-mode Interrupts Enable = 0 and WQ PASID Enable = 1 and Priv = 0.</td></tr><tr><td>Completion Interrupt Handle</td><td>Request Completion Interrupt = 0.</td></tr><tr><td>Fence</td><td>Descriptor submitted directly to WQ (not in a batch).</td></tr><tr><td>Block On Fault</td><td>WQ Block On Fault Enable = 0.</td></tr><tr><td>Destination Readback</td><td>GENCAP Destination Readback Support = 0.</td></tr><tr><td>Address 1 TC Selector</td><td>In Drain descriptor, if Readback Address 1 Valid = 0.</td></tr><tr><td>Address 2 TC Selector</td><td>In Drain descriptor, if Readback Address 2 Valid = 0.</td></tr><tr><td>Completion Record Address</td><td>Completion Record Address Valid = 0.</td></tr><tr><td>Request Completion Record</td><td>Completion Record Address Valid = 0.</td></tr><tr><td>Completion Record TC Selector</td><td>Completion Record Address Valid = 0.</td></tr><tr><td>Cache control flags</td><td>See section 8.1.3.1 for combinations of cache control flags that are reserved.</td></tr><tr><td>Readback Address 1 Valid</td><td rowspan="2">In Drain descriptor, if Drain Descriptor Readback Address Support = 0.</td></tr><tr><td>Readback Address 2 Valid</td></tr><tr><td>Use Alternate Source PASID</td><td rowspan="2">In Type Conversion, Gather Reduce, or scatter-gather descriptors, if the corresponding bit in the Operations with Inter-Domain Support field in DSACAP0 = 0.</td></tr><tr><td>Use Alternate Destination PASID</td></tr><tr><td>Use Inter-Domain Selector</td><td>In Reduce or Reduce with Dualcast descriptors, if the corresponding bit in the Operations with Inter-Domain Support field in DSACAP0 = 0.</td></tr><tr><td>Source/Source1 IDPT Handle</td><td>In inter-domain, scatter-gather, or Type Conversion descriptors, if Use Alternate Source/Source1 PASID = 0.</td></tr><tr><td>Destination/Source2 IDPT Handle</td><td>In inter-domain, scatter-gather, or Type Conversion descriptors, if Use Alternate Destination/Source2 PASID = 0.</td></tr><tr><td>Inter-Domain Selector</td><td rowspan="3">In Reduce or Reduce with Dualcast descriptors, if Use Inter-Domain Selector = 0.</td></tr><tr><td>IDPT Handle1</td></tr><tr><td>IDPT Handle2</td></tr><tr><td>Window Mode</td><td>In Update Window descriptor, if Window Enable = 0 or if Offset Mode Support in IDCAP = 0.</td></tr><tr><td>Window Base Address</td><td rowspan="2">In Update Window descriptor, if Window Enable = 0.</td></tr><tr><td>Window Size</td></tr></table>

Table 5-4: Conditional Reserved Field Checking

<table><tr><td>Operation</td><td>Required Flags (Must be 1)</td></tr><tr><td>Drain</td><td>Either Request Completion Record or Request Completion Interrupt must be set to 1.</td></tr><tr><td>Compare</td><td rowspan="6">Completion Record Address Valid and Request Completion Record flags must be 1.</td></tr><tr><td>Compare Pattern</td></tr><tr><td>Create Delta Record</td></tr><tr><td>CRC Generation</td></tr><tr><td>Copy with CRC Generation</td></tr><tr><td>DIF Check</td></tr><tr><td>Inter-Domain Copy</td><td>Either Use Alternate Source PASID must be 1 or Use Alternate Destination PASID must be 1.</td></tr><tr><td>Inter-Domain Fill</td><td>Use Alternate Destination PASID must be 1.</td></tr><tr><td>Inter-Domain Compare</td><td>Either Use Alternate Source1 PASID must be 1 or Use Alternate Source2 PASID must be 1.</td></tr><tr><td>Inter-Domain Compare Pattern</td><td>Use Alternate Source PASID must be 1.</td></tr></table>

Table 5-5: Operation Types with Required (Must be 1) Flags

# 5.6 Inter-Domain Permissions Table Entry Checks

The device performs the following checks while processing a descriptor that references an IDPT entry:

- Any IDPTE referenced in a descriptor must satisfy the following:

o Reserved fields are 0 as described in section 9.2.29. (This includes fields that are conditionally reserved).

The type field in the IDPTE is one of the supported types in IDCAP.

If Window Enable is 1:

- Window Base plus Window Size is less than or equal to  $2^{64}$  (i.e., the window does not wrap around the  $2^{64}$  address boundary).

Window Size is not zero.

- For any inter-domain descriptor:

The Usable field is 1 in each IDPTE referenced by the descriptor.

o Read permission is 1 in an IDPTE corresponding to a source address.

Write permission is 1 in an IDPTE corresponding to a destination address.

If IDPTE Type is 0, the descriptor PASID matches the submitter PASID field in the IDPTE.

If IDPTE Type is 1, the bit corresponding to the descriptor PASID is 1 in the submitter bit-map in memory.

If Window Enable in the IDPTE is 1 and Window Mode is 0, then the address range in the descriptor referencing the IDPTE lies within the range specified by the window base and size; specifically:

- Address is greater than or equal to Window Base, and less than Window Base plus Window Size.

- Address plus transfer size is greater than Window Base, and less than or equal to Window Base plus Window Size.

If Window Enable in the IDPTE is 1 and Window Mode is 1, then address is less than Window Size and address plus transfer size is less than or equal to Window Size.

- For a Type Conversion, Reduce, Reduce with Dualcast, or Gather Reduce descriptor, if Window Enable is 1 and Window Mode is 1, then Window Base is aligned to the size of the data type in the descriptor.

For an Update Window descriptor:

The Allow Update bit in the IDPTE is 1.

The descriptor PASID matches the access PASID field in the IDPTE.

# 5.7 Device Halt State

In addition to its normal states of operation, Intel DSA has a halt state to deal with various error or unsupported conditions and reset transitions. The device may enter halt state as a means for error containment, to prevent further propagation of an error. Software can find out the current device state by reading the Device State field of the General Status register. GENSTS also indicates the type of reset required to recover from the devicehalt condition. Based on this, software determines how to reset the device and bring it to a Normal mode of operation. If the Halt State Interrupt Enable field in GENCTRL is 1, an interrupt using entry 0 of the MSI-X table is generated when the device enters halt state. The Halt State field in the INTCAUSE register is set to 1 to indicate the interrupt cause to software. Some of the causes that may result in the device entering the halt state include:

-Unsupported PCIe configuration changes (for example, setting BME to 0).

- Parity error on a register write or certain internal buffers.

- Severe I/O fabric error (e.g., parity error encountered on transaction received over internal I/O fabric).

Note that not all errors result in the device entering this state, and most errors are handled without causing the device to Halt. It may also be noted that parity errors on data are normally reported and handled via PCIe AER mechanism and are not considered a severe I/O error.

In this state, the device typically stops sending upstream reads and writes. Depending on the severity of error, the device may continue to send completions for non-posted requests (e.g., register reads). New descriptor submissions via ENQCMD or ENQCMDS receive a retry response. The device typically continues to send invalidation completions unless it has encountered a severe I/O fabric error or is actively going through a PCIe reset. An implementation may treat configuration registers that are read-write while the device is Disabled as read-write in the Halt state also.

This state requires some level of reset to restore the device to normal operation. The type of reset needed (Reset Device command, Function-level reset, warm reset, or cold reset) is indicated by the Reset Type Required field in the GENSTS register (section 9.2.10), which indicates the minimum reset type needed to recover. Software can choose to invoke a stronger type of reset to reinitialize the device. The mechanism used to trigger a warm reset or cold reset may be platform-specific. When using Function Level Reset, software is expected to follow the app note in the PCIe specification, section 6.6.2.

# 5.8 Error Codes

# 5.8.1 Operation Status Codes

The operation status code for a descriptor is written to the Status field of the completion record for the descriptor if a valid completion record is available for the descriptor. If the operation status is 0xla, 0x1b, or 0x1d, or if the Completion Record Address Valid Flag is 0 and the operation status is not equal to 0x01, 0x02, or 0x05, then the operation status code is instead written either to the SWERROR register or to the Event Log if enabled.

<table><tr><td>0x01</td><td>Success.</td></tr><tr><td>0x02</td><td>Success with false predicate.</td></tr><tr><td>0x03</td><td>Partial completion due to page fault, when the Block on Fault flag in the descriptor is 0.</td></tr><tr><td>0x04</td><td>Partial completion due to: 
- An Invalid Request response to a Page Request. 
- Excessive Page Request retries for the same address without a successful translation.</td></tr><tr><td>0x05</td><td>One or more operations in the batch completed with Status not equal to Success. This value is used only for a Batch descriptor.</td></tr><tr><td>0x06</td><td>Partial completion of batch due to page fault while translating the Descriptor List Address in a Batch descriptor and: 
- Page Request Services are disabled; 
- WQ PRS Disable is 1; or 
- An Invalid Request response was received for the Page Request for the Descriptor List Address. 
This value is used only for a Batch descriptor.</td></tr><tr><td>0x07</td><td>Offsets in the delta record were not in increasing order. This value is used only for an Apply Delta Record operation.</td></tr><tr><td>0x08</td><td>An offset in the delta record was greater than or equal to the Transfer Size of the descriptor. This value is used only for an Apply Delta Record operation.</td></tr><tr><td>0x09</td><td>DIF error. This value is used for the DIF Check, DIF Strip, and DIF Update operations.</td></tr><tr><td>0x0a - 0x0f</td><td>Unused.</td></tr><tr><td>0x10</td><td>Unsupported operation code.</td></tr><tr><td>0x11</td><td>Invalid flags. One or more flags in the descriptor Flags field contain an unsupported or reserved value.</td></tr><tr><td>0x12</td><td>Non-zero reserved field (other than a flag in the Flags field).</td></tr><tr><td>0x13</td><td>Invalid Transfer Size or Element Count 
- The Transfer Size or Element Count is 0 in a descriptor that requires a non-zero value. 
- The Transfer Size field in the descriptor is greater than the WQ Maximum Transfer Size field in WQCFG. 
- The Transfer Size in a Create Delta Record or Apply Delta Record descriptor is greater than the allowed value, as described in section 8.3.8. 
- In a Type Conversion, Reduce or Reduce with Dualcast operation, the transfer size implied by the product of Element Count and the size of either data type is greater than the WQ Maximum Transfer Size field in WQCFG.</td></tr><tr><td></td><td>- In a Gather Reduce operation, the block size is greater than Maximum Supported Gather Reduce Block Size in DSACAP0, or the total number of bytes represented by the SGL entries is greater than the WQ Maximum Transfer Size in WQCFG. - In a Gather Copy, Scatter Copy, or Scatter Fill operation, the Transfer Size is not equal to the total number of bytes represented by the SGL entries.</td></tr><tr><td>0x14</td><td>Descriptor Count or SGL Size out of range - Descriptor Count for a Batch descriptor is zero or greater than the maximum batch size for the WQ. - Batch1 Support in GENCAP is 0 and Descriptor Count for a Batch descriptor is 1. - SGL Size for a scatter-gather operation is zero or is greater than the maximum SGL size for the WQ.</td></tr><tr><td>0x15</td><td>Unsupported value for a descriptor field other than the descriptor Flags field. - Invalid Maximum Delta Record Size or Delta Record Size for a Create Delta Record or Apply Delta Record operation. - Invalid Compute Type, Compute Flags, Data Type, or combination of input and output data types for a Type Conversion, Reduce, Reduce with Dualcast, or Gather Reduce operation. - Invalid SGL format type for operations that specify a Scatter-Gather List. -Unsupported value in the Inter-Domain selector field for a Reduce or Reduce with Dualcast operation. -Invalid Data Type for a Gather Copy, Scatter Copy, or Scatter Fill operation.</td></tr><tr><td>0x16</td><td>Overlapping buffers.</td></tr><tr><td>0x17</td><td>Bits 1:0 of the two destination buffers differ in Memory Copy with Dualcast or Reduce with Dualcast.</td></tr><tr><td>0x18</td><td>Misaligned Descriptor List Address.</td></tr><tr><td>0x19</td><td>Invalid Completion Interrupt Handle. - If the Request Interrupt Handle command is not supported: o The handle is out of range of the MSI-X or IMS table. - If the Request Interrupt Handle command is supported: o The interrupt handle was not returned by the Request Interrupt Handle command. o The interrupt handle has been revoked. See section 3.7. - The PASID Enable and PASID fields in the selected interrupt table entry don't match those of the descriptor.</td></tr><tr><td>0xla</td><td>A page fault occurred while translating a Completion Record Address and: - Page Request Services are disabled; - WQ PRS Disable is 1; or - An Invalid Request response was received for the Page Request for the completion record.</td></tr><tr><td>0x1b</td><td>Completion Record Address is not 32-byte aligned.</td></tr><tr><td>0x1c</td><td>Misaligned address, size, or stride field: - In a Create Delta Record or Apply Delta Record operation: Source1 Address, Source2 Address, Destination Address, or Transfer Size is not 8-byte aligned. - In a CRC Generation or Copy with CRC Generation operation: CRC Seed Address is not 4-byte aligned. - In a Translation Fetch operation: Region Stride is less than 4096 or is not a power of 2. - In a DIX Generate operation: Destination Address is not 8-byte aligned.</td></tr><tr><td></td><td>- In a Type Conversion, Reduce, Reduce with Dualcast, or Gather Reduce operation, a source or destination address is not naturally aligned to the size of the corresponding data type specified in the descriptor.</td></tr><tr><td>0x1d</td><td>In a descriptor submitted to an SWQ, Priv is 1 and the Privileged Mode Enable field of the PCI Express PASID capability is 0.</td></tr><tr><td>0x1e</td><td>Incorrect Traffic Class configuration: 
- A TC selected by the descriptor is not enabled in the TC/VC Map of any VC Resource Control register. 
- A TC selected by the descriptor is enabled in the TC/VC Map of a VC Resource Control register in which VC Enable is 0.</td></tr><tr><td>0x1f</td><td>A page fault occurred while translating a Readback Address in a Drain descriptor and: 
- Page Request Services are disabled; 
- WQ PRS Disable is 1; or 
- An Invalid Request response was received for the Page Request for the Drain Readback Address.</td></tr><tr><td>0x20</td><td>The operation failed due to a hardware error not covered by error code 0x21 or 0x22. For example: 
- A UR or CA response or completion timeout on a read operation. 
- An address translation fault on a read operation when ATS is disabled. 
- A hardware error such as a parity error. Details of the hardware error are reported via PCIe Advanced Error Reporting (AER), if enabled.</td></tr><tr><td>0x21</td><td>Hardware error, including a UR or CA response or completion timeout, on a memory write1 or destination readback operation. Error details are reported via PCIe Advanced Error Reporting (AER), if enabled.</td></tr><tr><td>0x22</td><td>Error during address translation: 
- A UR or CA response or a completion timeout (CTO) on an ATS translation request. 
- A Response Failure response to a Page Request. The error is also recorded in SWERROR and in some cases, also via PCIe Advanced Error Reporting (AER), if enabled.</td></tr><tr><td>0x23 -0x25</td><td>Unused.</td></tr><tr><td>0x26</td><td>Hardware may have encountered a page fault on the completion record for one or more descriptors prior to a Drain descriptor. This error is only reported in an Event Log entry for a Drain descriptor (described in section 5.9).</td></tr><tr><td>0x27</td><td>Hardware may have encountered a page fault on the completion record for one or more descriptors in a batch. This error is only reported in an Event Log entry for a Batch descriptor (described in section 5.9).</td></tr><tr><td>0x28</td><td>An inter-domain or Update Window operation was submitted to a WQ that has WQ PASID Enable as 0.</td></tr><tr><td>0x29</td><td>Invalid IDPTE handle: An inter-domain operation referenced an IDPTE that is invalid, out-of-range, not usable, of a type not supported by the operation, or inaccessible to the submitter. Refer to section 5.6 for a list of IDPT entry checks.</td></tr><tr><td>0x2a</td><td>An inter-domain operation failed due to insufficient permissions for the requested access. 
- A read operation using an IDPTE that did not grant read permissions. 
- A write operation using an IDPTE that did not grant write permissions.</td></tr><tr><td>0x2b</td><td>An inter-domain operation failed window address range checks.</td></tr><tr><td>0x2c</td><td>An Update Window operation referenced an IDPTE that is invalid, out-of-range, not modifiable, or not owned by the submitter. Refer to section 5.6 for a list of IDPT checks.</td></tr><tr><td>0x2d</td><td>Invalid window control fields in an Update Window descriptor 
- The Window Mode flag in the descriptor is set to enable Offset Mode when the Window Enable flag in the descriptor is 0 or Offset Mode Support in IDCAP is 0. 
- The Window Enable flag in the descriptor is 1, but Window Size is 0. 
- Window Base plus Window Size is greater than 264.</td></tr><tr><td>0x2e</td><td>An inter-domain operation failed because the requested domain was inaccessible. Hardware reports this error code when terminating an inter-domain operation while executing an Abort command.</td></tr><tr><td>0x2f - 0x3f</td><td>Unused.</td></tr></table>


Table 5-6: Operation Status Codes


# 5.8.2 Other Software Error Codes

<table><tr><td>0x51</td><td>An unsupported change was made to one of the registers in PCI configuration space while the device was not Disabled. This causes the device to enter the Halt State. This error is only reported in the SWERROR register.</td></tr><tr><td>0x52</td><td>The Command register was written while the Active field of the Command Status register was 1. This error is reported either in the SWERROR register or in the Event Log if enabled.</td></tr><tr><td>0x53</td><td>A descriptor was submitted to a dedicated WQ that had no space to accept the descriptor. This error is only reported in the SWERROR register.</td></tr><tr><td>0x54</td><td>The device was unable to append an event to the Event Log. This error may occur because the Event Log was full and Event Log Overflow Support in GENCAP is 1. It may also occur due to a page fault, UR, or CA while writing to the Event Log. This error is only reported in the SWERROR register.</td></tr></table>


Table 5-7: Other Software Error Codes


# 5.8.3 Administrative Command Error Codes

These errors are reported in the Command Status register (described in section 9.2.13).

<table><tr><td>Command</td><td>Error codes</td></tr><tr><td>Enable Device</td><td>0x10: Device is not Disabled.0x11: Unspecified error in configuration when enabling the device.0x12: Bus Master Enable is 0.0x13: PRSREQALLOC is configured with an unsupported value.0x14: Sum of WQCFG Size fields is out of range.0x15: Invalid Group configuration:- A Group Configuration register has one or more WQs and zero engines or has one or more engines and zero WQs.</td></tr><tr><td></td><td>- A Group Configuration register beyond the number of groups contains non-zero fields.0x16: Invalid Group configuration:- A WQ is in more than one group.- An active WQ (with non-zero WQ Size) is not in a group.- An inactive WQ is in a group.- Reserved bits are set in the WQs field of a Group Configuration Register.0x17: Invalid Group configuration:- An engine is in more than one group.- Reserved bits are set in the Engines field of a Group Configuration Register.0x18: Invalid Read Buffers or Bandwidth Limit configuration:- Invalid value for Global Read Buffer Limit in GENCFG or Use Global Read Buffer Limit in GRPCFG.- Invalid value for Read Buffers Allowed or Read Buffers Reserved in GRPCFG.- Invalid value for Read Bandwidth Limit or Write Bandwidth Limit in GRPCFG.0x19: Invalid Event Log configuration:- Event Log Enable in GENCFG is 1 when Event Log Support in GENCAP is 0.- Event Log Size &lt; 64.- Event Log Base Address + Event Log Size × Event Log entry size &gt;264.0xla: Invalid ATS configuration:- ATS Enable is 0 and ATS is required by the platform.- ATS Enable is 1 and ATS is not supported in the platform.0x1b: Invalid PASID configuration:- EVLCFG PASID Enable is 1 and the PASID Enable field of the PCI Express PASID capability is 0.- EVLCFG Priv is 1 and EVLCFG PASID Enable is 0 or the Privileged Mode Enable field of the PCI Express PASID capability is 0.- IDBR PASID Enable is 1 and the PASID Enable field of the PCI Express PASID capability is 0.- IDBR Priv is 1 and IDBR PASID Enable is 0 or the Privileged Mode Enable field of the PCI Express PASID capability is 0.</td></tr><tr><td>Enable WQ</td><td>0x20: Device is not Enabled.0x21: WQ is not Disabled.0x22: WQ Size is 0.Note: WQ Size out of range is diagnosed when the device is enabled.0x23: WQ Priority is 0.0x24: Invalid WQ mode:- WQ Mode = 0 and WQCAP Shared Mode Support = 0.- WQ Mode = 1 and WQCAP Dedicated Mode Support = 0.</td></tr><tr><td></td><td>0x25: WQ Block on Fault Enable = 1 and either the Block on Fault Support field in GENCAP is 0 or the Enable field of the PCIe Page Request Control register is 0.0x26: Invalid value for WQ PASID Enable:- WQ PASID Enable = 0 and WQ Mode = 0.- WQ PASID Enable = 1 and PCI Express PASID capability Enable = 0.0x27: Invalid WQ Maximum Batch Size or WQ Maximum SGL Size- WQ Maximum Batch Size is 0 or greater than Maximum Supported Batch Size.- WQ Maximum SGL Size is greater than Maximum Supported SGL Size.0x28: Invalid WQ Maximum Transfer Size- WQ Maximum Transfer Size greater than Maximum Supported Transfer Size.0x2a: WQ Mode = 1, WQ PASID Enable = 1, WQ Priv = 1, and the Privileged Mode Enable field of the PCI Express PASID capability = 0.0x2b: Invalid value for WQ Occupancy Interrupt Table or WQ Occupancy Interrupt Handle.0x2c: WQ ATS Disable = 1 and the WQ ATS Support field in WQCAP is 0.0x2d: Invalid WQ OPCFG; i.e., WQ OPCFG AND ~OPCAP is not 0.0x2e: WQ PRS Disable = 1 and:- WQ PRS Support field in WQCAP is 0;- Event Log Enable in GENCFG is 0; or- WQ Block on Fault Enable is 1.</td></tr><tr><td>Disable Device</td><td>0x31: Device is not Enabled.</td></tr><tr><td>Disable WQDrain WQAbort WQ</td><td>0x32: One or more of the specified WQs are not Enabled.</td></tr><tr><td>Reset WQReset DeviceDrain AllAbort AllDrain PASIDAbort PASID</td><td>No error codes are defined for these commands.</td></tr><tr><td>Request Interrupt Handle</td><td>0x41: Invalid interrupt table index.0x42: No handle is available.</td></tr><tr><td>Release Interrupt Handle</td><td>0x41: Invalid interrupt table index.</td></tr><tr><td>Request IDPT Handle</td><td>0x51: Invalid Inter-Domain Permissions Table index.0x52: No handle is available.</td></tr><tr><td>Release IDPT Handle</td><td>0x51: Invalid Inter-Domain Permissions Table index.</td></tr></table>

Table 5-8: Administrative Command Error Codes

# 5.9 EventLog

Errors on the completion record of a descriptor or during processing of a descriptor that does not have a valid completion record address are normally reported in the Software Error Register. Occurrence of multiple such errors before software has processed the Software Error Register results in an overflow condition. As an alternative, if the value of the Event Log Support field in GENCAP is not 0, hardware supports logging of such events in an Event Log in memory. Software specifies the address and size of the Event Log region in the EVLCFG register. The Event Log is enabled if the Event Log Enable bit in GENCFG is 1 when the device is enabled.

The size of entries in the Event Log is specified by the Event Log Support field in GENCAP. The format of Event Log entries is described in section 5.9.1. An implementation may issue Event Log writes as either Translated or Untranslated accesses. Software must pin memory pages corresponding to the Event Log. Event Log writes are performed with a TC value of 0. If the PASID Enable field in EVLCFG is 1, Event Log writes are issued as writes with PASID using the PASID and privilege specified in EVLCFG. If Event Log is enabled, the device initializes the head and tail fields in EVLSTATUS when the device is enabled. Once enabled, hardware writes each event to the offset specified by the Event Log Tail field in EVLSTATUS and increments the tail value. When the tail reaches the end of the log, it wraps to 0. The next event to be processed by software is specified by the Event Log Head field in EVLSTATUS. Software updates the head field after processing one or more events at the head of the Event Log. The log is full when tail + 1 mod log-size = head.

At the time of writing an event to the log, if the Event Log Interrupt Enable field in GENCTRL is 1 and the Interrupt Pending bit in EVLSTATUS is 0, hardware sets the Interrupt Pending bit in EVLSTATUS to 1, sets the Event Log field of the Interrupt Cause register to 1, and generates an interrupt using MSI-X entry 0. No further interrupts are generated for additional log entries until software clears the Interrupt Pending bit.

If Event Log Overflow Support in GENCAP is 0 and the Event Log is full when hardware tries to append an event, hardware blocks until software updates the Event Log Head field after processing one or more events at the head of the log. Hence software must ensure that the Event Log region in memory is adequately sized, and that Event Log entries are processed in a timely manner. If Event Log Overflow Support in GENCAP is 1 and the Event Log is full when hardware tries to append an event, the event is dropped and hardware attempts to log an error in the SWERROR register to indicate the event log full condition. If the Valid bit in the SWERROR register is already 1, the behavior is as described in section 9.2.15.

If hardware encounters a page fault on a completion record address while PRS is disabled<sup>1</sup>, it is reported as an error. If the Event Log is enabled, hardware writes an entry to the Event Log with an appropriate error code indicating the cause of the page fault; otherwise, it is reported via SWERROR. In the former case, hardware also writes the completion record for that descriptor to the Event Log entry. Completion records written to the Event Log have the same format as described in sections 8.2 and 8.3. Software responsible for processing the Event Log should propagate the completion record to the software entity

that submitted the faulting descriptor. Software should also generate the completion interrupt as indicated by the Error Information field in the Event Log entry.

Event Log entries are written in the order in which descriptors are completed by the engines, as described in section 3.9. If the completion record for any descriptor in a batch is written to the Event Log due to a page fault on the completion record address, the completion for the corresponding Batch descriptor is also written to the Event Log, if either a completion record or a completion interrupt is required for the Batch descriptor. An Event Log entry for a Batch descriptor is written only after any completion records and Event Log entries for descriptors in the batch are written. In this case, the error code in the Event Log entry for the batch descriptor indicates that one or more descriptors in the batch have associated Event Log entries with completion records that must be processed by software.

Hardware generates a batch identifier value to allow software to correlate Event Log entries for descriptors within a batch and for the corresponding Batch descriptor and is reported in the Batch Identifier field of an Event Log entry. A batch identifier may be reused by hardware once the Event Log entry for the Batch descriptor has been written. An Event Log entry with the First Error in Batch flag as 1 identifies the first entry for that batch. This allows software to identify any stale Event Log entries with the same batch identifier. If software encounters an entry with this flag as 1, any outstanding page faults previously recorded for the same batch identifier should be discarded.

The completion for a successful Drain descriptor is written to the Event Log if it is enabled, and if either the Enable bit in PRSCTL is 0 or if WQ PRS Disable in WQCFG is 1. If Event Log is not enabled, or if PRS is enabled, or if a Drain descriptor is not executed because of a descriptor check failure or a page fault, the Drain descriptor completion record is written to the completion record address.<sup>1</sup>

# 5.9.1 Event Log Entry

The format of each entry is shown below. Any bytes in an Event Log entry beyond those specified here, and up to the entry size specified in GENCAP are reserved.

![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/1d0c9667edc782126ca82056968a4edb203bf3d30d7d5caa702a5a78dcb9b83d.jpg)



Figure 5-1: Event Log Entry


<table><tr><td>Byte Offset</td><td>Bits</td><td>Size</td><td colspan="2">Description</td></tr><tr><td rowspan="13">7:0</td><td>63:60</td><td>4 bits</td><td colspan="2">Unused.</td></tr><tr><td>59:40</td><td>20 bits</td><td colspan="2">PASID
The PASID field of the descriptor that caused the error.</td></tr><tr><td>39:32</td><td>8 bits</td><td colspan="2">Operation
The Operation field of the descriptor that caused the error.</td></tr><tr><td>31:24</td><td>8 bits</td><td colspan="2">Batch Identifier
Identifier used to correlate Event Log entries for descriptors in a batch with the Event Log entry for the Batch descriptor.
This field is valid if the Descriptor Valid field is 1 and either the Batch Member field is 1 or the Operation field is 0x01 (Batch operation).
Otherwise, this field is unused.</td></tr><tr><td>23:16</td><td>8 bits</td><td colspan="2">WQ Index
Indicates which WQ that the descriptor was submitted to.</td></tr><tr><td>15:8</td><td>8 bits</td><td colspan="2">Error code
See section 5.8 for the meaning of the value in this field.</td></tr><tr><td>7</td><td>1 bit</td><td colspan="2">Error Information Valid
0: Error Information field is valid only if the Error code is Invalid Flags (0x11).
1: Error Information field is valid and provides additional information pertaining to the error reported in the Error code field.</td></tr><tr><td>6</td><td>1 bit</td><td colspan="2">Priv
The Priv field of the descriptor that caused the error.</td></tr><tr><td>5</td><td>1 bit</td><td colspan="2">R/W
If the error is a page fault, this indicates whether the faulting access was a read or a write.
0: The faulting access was a read.
1: The faulting access was a write.
Page faults are indicated by error codes 0x03, 0x04, 0x06, 0x1a, and 0x1f. For other error code values, this field is unused.</td></tr><tr><td>4</td><td>1 bit</td><td colspan="2">Batch Member
0: The descriptor was submitted directly.
1: The descriptor was submitted in a batch.</td></tr><tr><td>3</td><td>1 bit</td><td colspan="2">WQ Index Valid
0: The WQ that the descriptor was submitted to is unknown. The WQ Index field is unused.
1: The WQ Index field indicates which WQ the descriptor was submitted to.</td></tr><tr><td>2</td><td>1 bit</td><td colspan="2">Descriptor Valid
0: The descriptor that caused the error is unknown. The Batch Member, Operation, Batch Index, Priv, and PASID fields are unused.
1: The Batch Member, Operation, Batch Index, Priv, and PASID fields are valid.</td></tr><tr><td>1:0</td><td>2 bits</td><td colspan="2">Unused.</td></tr><tr><td>15:8</td><td>63:32</td><td>32 bits</td><td colspan="2">Error Information</td></tr><tr><td>Byte Offset</td><td>Bits</td><td>Size</td><td colspan="2">Description</td></tr><tr><td rowspan="8"></td><td rowspan="8"></td><td rowspan="8"></td><td colspan="2">This field reports additional information for the error codes listed below. Otherwise, this field is unused.</td></tr><tr><td>Error code</td><td>Error information</td></tr><tr><td>Invalid Flags(0x11)</td><td>63:32 – A bitmask of the flags that were found to be invalid. If a bit in this field is 1, it indicates that the flag at the corresponding bit position in the Flags field of the descriptor was invalid.</td></tr><tr><td>Invalid Interrupt Handle(0x19)</td><td>63:52 – Unused.51 – First Error in Batch.50 – Completion Record Required.49 – Portal. 0: MSI-X portal; 1: IMS portal.48 – Completion Interrupt Required.(Always 1.)47:32 – Interrupt handle</td></tr><tr><td>Page Fault(0x03,0x04)</td><td>63:61 – Operand Identifier. See Table 8-13 for a description of this field.For Inter-domain operations:60:48 – Unused.47:32 – The IDPT handle used with the faulting address.For other operation types, bits 60:32 are unused.</td></tr><tr><td>Page Fault(0x06,0x1f)</td><td>63:61 – Operand Identifier. See Table 8-13 for a description of this field.60:32 – Unused.</td></tr><tr><td>Page Fault(0x1a)</td><td>63:61 – Operand Identifier. See Table 8-13 for a description of this field.60:52 – Unused.51 – First Error in Batch.50 – Completion Record Required.(Always 1.)49 – Portal. 0: MSI-X portal; 1: IMS portal.Undefined if Completion Interrupt Required is 0.48 – Completion Interrupt Required.47:32 – Interrupt handle (if bit 48 is 1).</td></tr><tr><td>Page Fault before Drain or in a batch(0x26,0x27)</td><td>63:51 – Unused.50 – Completion Record Required.49 – Portal. 0: MSI-X portal; 1: IMS portal.Undefined if Completion Interrupt Required is 0.48 – Completion Interrupt Required.47:32 – Interrupt handle (if bit 48 is 1).</td></tr></table>

<table><tr><td>Byte Offset</td><td>Bits</td><td>Size</td><td colspan="3">Description</td></tr><tr><td rowspan="3"></td><td></td><td></td><td>Inter-Domain operation error (0x29-0x2c)</td><td>63:48 – Unused. 47:32 – The IDPT handle that caused the error.</td><td></td></tr><tr><td>31:16</td><td>16 bits</td><td colspan="3">Unused.</td></tr><tr><td>15:0</td><td>16 bits</td><td colspan="3">Batch Index If the Descriptor Valid field is 1 and the Batch Member field is 1, this field contains the index of the descriptor within the batch. Otherwise, this field is unused.</td></tr><tr><td>23:16</td><td>63:0</td><td>64 bits</td><td colspan="3">Error Log Address If error code is 0x03, 0x04, 0x06, or 0x1f, this is the faulting address. Bits 11:0 may be reported as 0. If error code is 0x1a, 0x26, or 0x27, this is the completion record address, and all bits of the address are reported. Otherwise, this field is unused.</td></tr><tr><td>31:24</td><td>63:0</td><td>64 bits</td><td colspan="3">Unused.</td></tr><tr><td>63:32</td><td>255:0</td><td>256 bits</td><td colspan="3">Completion Record If error code is 0x19, 0x1a, 0x26, or 0x27 and Completion Record Required is 1, this field contains the completion record for the operation. Otherwise, this field is unused.</td></tr></table>

Table 5-9: Event Log Entry Format

# 6 Performance Monitoring

The purpose of the Intel DSA performance monitoring capability (perfmon) is to support collection of information about key events (architectural or micro-architectural) occurring during device execution, to aid performance tuning and debug. This can also be useful to understand usages of key features and operations supported by the device. The perfmon architecture comprises three parts:

- Ability to discover and enumerate perfmon capabilities supported by a given Intel DSA implementation.

- Set of configuration and data registers to enable and configure the device to monitor a subset of supported events.

- List of events and filters supported.

Details of the registers used to enumerate and configure the various perfmon capabilities can be found in section 9.2.25.

# 6.1 Perfmon Discovery and Enumeration

Software reads a set of capability registers to discover whether the device supports the perfmon capability, and if so, to enumerate details of the capability such as number of counters, counter width, event categories, filter support, etc. If an implementation does not support performance monitoring, then the performance capability (PERFCAP) register is reported as 0 and the other perfmon capability, configuration, and data registers are not supported.

Software configures a counter to monitor events by specifying two pieces of information: an Event Category and an Events field, in the counter configuration register (CNTRCFG). The Intel DSA perfmon architecture defines a set of architectural Event Categories along with a set of implementation-specific events for each Event Category. The Event Categories are defined in Table 6-1. Additional Event Categories may be added in future implementations.

Corresponding to each Event Category defined, there is an event capability register (EVNTCAP) to report the set of events supported in that category. In case an implementation does not support any events for a given Event Category, then the Events field in EVNTCAP is reported as zero, and software should not attempt to configure any counter with that category. Details of the events corresponding to each Event Category are in Appendix D. When enabling a counter to count events of a given category, bits corresponding to events not supported in the specified event category are ignored. Software should not rely on this behavior since a future implementation may support additional events in an event category, resulting in bits that are ignored in one implementation having a defined meaning in a different implementation.

The mapping of Event Categories and Events to Event Counters may be implementation specific. An implementation may allow any event to be counted by any counter. However, an implementation may also choose to restrict this by allowing only certain Event Categories to be counted by each Event Counter. In such cases, software can consult the per-counter capability register (CNTRCAP) to discover the Event Categories and sets of events supported by each counter.

<table><tr><td>Value</td><td>Event Category</td><td>Description</td></tr><tr><td>0</td><td>WQ</td><td>Specifies events pertaining to work submission to a shared or dedicated work queue.</td></tr><tr><td>1</td><td>Engine</td><td>Specifies events pertaining to dispatch of work descriptors from the WQs and execution of the descriptors in the corresponding engines.</td></tr><tr><td>2</td><td>Address Translation</td><td>Specifies events pertaining to address translation when processing descriptors (including ATS/PRS and invalidation related events).</td></tr><tr><td>3</td><td>Operations</td><td>Counts operations of a specified type.</td></tr><tr><td>4</td><td>Completions</td><td>Specifies events related to descriptor completion and interrupt generation.</td></tr><tr><td>5</td><td>Operations 2</td><td>Counts operations of a specified type.</td></tr><tr><td>6-15</td><td>Reserved</td><td>Reserved for future use.</td></tr></table>

Table 6-1: Event Categories

# 6.2 Perfmon Configuration Registers

When perfmon is supported, there is a set of configuration, status, and data registers that software can use to configure and control the perfmon hardware. This includes a set of global configuration and status registers, as well as per-counter configuration and data registers documented in section 9.2.25. Software can reset the state of all the supported counter control and data registers to the default initial values using the Reset Perfmon Configuration and Reset Perfmon Counter controls in PERFRST (see section 9.2.25.4). This may be done at any time. Reset of Perfmon Configuration results in all the counters being Disabled.

Each set of counter configuration and data registers operates independently, and software must configure and enable a counter before that counter can begin counting events. If event filtering is desired, software should program the Filter Configuration (FLTCFG) registers (see section 6.4) before enabling the corresponding counter configuration (CNTRCFG) register. While the Enable bit in CNTRCFG is set, writes to FLTCFG or other fields of CNTRCFG are ignored, and event monitoring continues as if the write did not happen. Writes to the counter data (CNTRDATA) registers are handled as described in section 6.3.

Software configures event counting by selecting an available counter register and programming the appropriate Event Category value and set of events to be monitored in the Events field in the corresponding counter configuration register (CNTRCFG). The device interprets the bits programmed in the Events field as corresponding to the specified Event Category. Hence, at any time, a given counter can only count events corresponding to a single Event Category. Software can configure a given counter to count multiple events belonging to the same Event Category by setting multiple bits in the Events field. In this case, the counter value reflects the sum of all occurrences of the specified events. If independent (non-additive) counts are required for some events, software needs to program different

counters; one per event to be monitored. Similarly, to count events corresponding to different Event Categories, software needs to configure multiple counters; at least one per Event Category desired, and with the corresponding Events value.

# 6.3 Event Counters

The perfmon architecture supports up to 32 event counters. The number of counters in a given implementation may be less than this. Software can discover the number of counters in a given implementation by reading the PERFCAP register. Each counter operates independently.

Software can read the counter data registers (CNTRDATA) at any time. Software can write to a CNTRDATA register prior to enabling the counter. If the Counters Writeable While Enabled field in PERFCAP is 1, writes to a CNTRDATA register are also allowed while the counter is enabled. Some usages of software writing to a counter data register include:

- Write to a counter while it is disabled to initialize the counter with a specific value.

- Write to a counter after it overflows to re-initialize the counter. Note that the counter may be enabled and currently counting (not frozen).

- Write to a currently frozen counter to re-initialize it.

The supported width of the counter data registers is discoverable by reading the capability (PERFCAP) register. While this may be less than 64 bits in a given implementation, software is still allowed to write a full 64-bit value to the register without causing an error to be triggered. However, bits above the specified width are ignored by hardware. If per-counter capability reporting is supported (as indicated by PERFCAP), then the supported width of each counter is the value specified in the corresponding CNTRCAP register. This allows an implementation to support counters of different widths.

When multiple events are enabled in a given counter register, if multiple events occur simultaneously, then the counter value is incremented by more than 1 in a given cycle.

# 6.3.1 Counter Overflow

While enabled to count events, if an event occurrence causes the counter value to increment and roll over to or past zero, this is termed as a counter overflow. Upon overflow, the corresponding bit in the overflow status register (OVSTATUS) is set. If supported and enabled, an interrupt may also be generated (details below). Normally, the counter continues to count events and does not stop counting upon overflow. If supported, software can specify the Global Freeze on Overflow bit in the counter configuration register. If this bit is set for a counter, an overflow of that counter results in the freeze bits of all counters to be set in PERFFRZ. This forces all the counters to stop counting (freeze) and retain their current count value (until explicitly written or reset by software). The current freeze state of the counters is reported in PERFFRZ.

As mentioned above, since the counter data registers may be software writeable, software can treat the data register as a signed integer up to the supported width. To cause an overflow on the first occurrence of an event, software can write an initial count value of -1 (e.g., 0xFFFFFFF for a 32-bit counter) prior to enabling the event counting. This causes the first occurrence of the specified event, after enabling the counter, to cause an overflow of the counter value and trigger an interrupt (if enabled).

# 6.3.2 Counter Stop and Resume

Software can stop a counter that is enabled and counting events, by writing a 1 to the corresponding bit in the PERFFRZ register. This is referred to as a freeze operation on that counter and causes it to stop counting further events. Likewise, a counter that was previously frozen may be resumed by writing a 0 to the corresponding bit in PERFFRZ. This is referred to as an unfreeze operation on the counter and causes it to resume counting of configured events. When unfrozen, the counter continues to increment, starting from the current counter value at the time of the unfreeze operation.

Current freeze/unfreeze status of a counter is reported in the PERFFRZ register. These bits are Readable and Writeable by software. Additionally, hardware sets the freeze bits of all counters when any counter overflows if the overflowing counter has the Global Freeze on Overflow bit set.

# 6.4 Filter Support

The perfmon architecture allows software to specify a set of filters that can be used to constrain the counting of selected events based on one or more conditions specified in the Filter Configuration registers. When supported, there is a set of architecturally defined Filters as described in Table 6-2 and a corresponding set of Filter Configuration registers (one per Filter) for each perfmon counter. Software can discover support for filtering capability by querying the perfmon capability register (PERFCAP).

Each event might only support a subset of filter types or may not support filters at all. See Appendix D for information on which filters are allowed for each event. Software can specify one or more filters to apply to the events monitored by a given counter by programming the Filter Values in the corresponding Filter Configuration registers (FLTCFG) for that counter. An example use of filters might be to configure a counter to only count a specific event, e.g., number of drain descriptors (specified via the Event_Category and Events fields), from only a specific WQ (filter).

Software is allowed to specify multiple filters for a given counter. When multiple filters are configured for a counter, only the events that satisfy all the specified filters (i.e., logical AND of all the filter conditions) will be counted. See section 0 for examples.

# 6.5 Event Programming Considerations

As mentioned in section 6.2, software can configure an event counter to count multiple events belonging to the same Event Category, by setting multiple bits in the Events field of the corresponding CNTRCFG register. To get meaningful event counts, software should ensure that when multiple events are to be monitored by a counter, the events are related in some way. For example, configuring a counter to count both number of cycles and number of operations may not be desirable. Similarly, all events within an Event Category may not support the same set of filters. Software should ensure that the filter values specified are compatible with the set of events configured for that counter. Not doing so may produce undesirable event counter values. Hardware does not perform error checks when programming the performance monitoring registers, and the onus is on software to ensure meaningful configuration.

<table><tr><td>Filter</td><td>Encoding</td><td colspan="3">Filter Value</td></tr><tr><td>WQ</td><td>0</td><td colspan="3">Bitmask to select WQs to monitor. 
(Bit 0 for WQ0, Bit 1 for WQ1, etc.)</td></tr><tr><td>Traffic Class 
(TC)</td><td>1</td><td colspan="3">Bitmask to select which TCs to monitor. 
(Bit 0 for TC0, Bit 1 for TC1, etc.)</td></tr><tr><td rowspan="5">Page Size</td><td rowspan="5">2</td><td colspan="3">Bitmask to select which Page Sizes to monitor</td></tr><tr><td>Bit</td><td>Filter Value</td><td>Description.</td></tr><tr><td>0</td><td>0x1</td><td>4K</td></tr><tr><td>1</td><td>0x2</td><td>2M</td></tr><tr><td>2</td><td>0x4</td><td>1G</td></tr><tr><td rowspan="10">Transfer Size</td><td rowspan="10">3</td><td colspan="3">Bitmask to select range of transfer size values to monitor.</td></tr><tr><td>Bit</td><td>Filter Value</td><td>Description</td></tr><tr><td>0</td><td>0x1</td><td>0 ≤ size &lt; 512B</td></tr><tr><td>1</td><td>0x2</td><td>512B ≤ size &lt; 2KB</td></tr><tr><td>2</td><td>0x4</td><td>2KB ≤ size &lt; 4KB</td></tr><tr><td>3</td><td>0x8</td><td>4KB ≤ size &lt; 16KB</td></tr><tr><td>4</td><td>0x10</td><td>16KB ≤ size &lt; 1MB</td></tr><tr><td>5</td><td>0x20</td><td>1MB ≤ size &lt; 64MB</td></tr><tr><td>6</td><td>0x40</td><td>64MB ≤ size &lt; 1GB</td></tr><tr><td>7</td><td>0x80</td><td>1GB ≤ size &lt; 4GB</td></tr><tr><td>Engine Number</td><td>4</td><td colspan="3">Bitmask to select which Engines to monitor.</td></tr><tr><td>PASID</td><td>5</td><td colspan="3">Select which submitter PASID to monitor: 
Bits 21:20: Filter mode. 
Bits 19:0: PASID value. 
Filter mode: 
0: Match requests with specified submitter PASID value. 
1: Match requests with any PASID. 
2: Match requests with no PASID. 
3: Match any request. (This is the default.)</td></tr></table>

Table 6-2: Filter Types and Mask

# 6.6 Interrupt Generation

If the Interrupt on Overflow Support field in PERFCAP is 1, then the implementation supports generation of an MSI-X interrupt (using entry 0 of the MSI-X table) upon counter overflow. Software can use this facility to be notified when a counter overflows.

Also, the INTCAUSE register indicates that a perfmon counter overflow caused the interrupt to be generated. Upon receiving the interrupt, software can read the global status register (OVFSTATUS) to identify which counters overflowed. It is possible for multiple bits to be set in this register (indicating multiple counter overflows). If Global Freeze on Overflow is enabled for the counter, software can check the current freeze state for all the counters in PERFFRZ and read the corresponding counter values from the CNTRDATA registers.

# 7 Reference Software Architecture

Software support for Intel DSA is expected to include the following elements:

- Kernel mode driver.

- User mode driver.

- Virtualization support.

# 7.1 Kernel Mode Driver

The Intel DSA kernel-mode driver (KMD) is responsible for initializing and managing the device. It can plug into the kernel DMA subsystem and provide services to any client using the internal OS-specific DMA APIs. It also exposes an interface to user space to support direct user level access for SVM services. KMD requests that the OS allocate/bind/unbind/free PASIDs based on user level requests. It maps limited portals to clients to allow them direct access for work submission. If the kernel does not allow mapping portals to user space, KMD should provide a software interface (e.g., a system call) to be used by clients for work submission.

For Shared WQs, KMD sets WQ Threshold to control how much of the WQ capacity is available for the limited portal. The remainder of the WQ capacity is reserved for work submission to the unlimited portal. KMD may change the threshold at any time. The threshold may be set to the WQ Size to not reserve any space and it may be set to 0 to prevent any work submission to the limited portal. When a limited portal returns Retry, the client can request that KMD submit work to the unlimited portal on its behalf. If the unlimited portal also returns Retry, KMD may reattempt the submission or take the following steps:

1. Reduce the threshold to prevent direct work submission.

2. Enable the WQ Occupancy interrupt to receive notification when there is space in the WQ.

3. When the notification arrives, submit the work that has been queued.

4. Restore the threshold.

In performing these steps, KMD may need to take care that descriptors are submitted to the device in the same order that they were attempted by the client, if the client relies on descriptors being executed in order. (See section 3.9 for information about descriptor ordering.)

If Event Log is supported, KMD is responsible for configuring and enabling it (as described in section 5.9). When notified of new Event Log entries written by hardware, KMD should read each entry in the log, handle the event, and update the Event Log Head field in EVLSTATUS. Handling events includes handling page faults on completion records as described in section 7.3. It may also include logging entries in a system log.

# 7.2 User Mode Driver

The Intel DSA user-mode driver (UMD) is an optional component that is used to provide user-mode access to the device. UMD is used to make Intel DSA functions available to applications. It is linked with an application as a library and interfaces with the kernel-mode driver to request access to the device on behalf of the application. It exposes various device functions to the application by abstracting them in higher level APIs. It normally services application requests using ENQCMD to a limited portal. If the ENQCMD fails due to congestion, UMD may back off and retry the work submission or use a kernel-

mode driver service to proxy the request to ensure forward progress. Additionally, UMD can service application requests using MOVDIR64B to a dedicated work queue portal.

# 7.3 Software Requirements for Handling Non-Blocking Page Faults

While PRS is disabled, Intel DSA handles page faults by stopping the operation and reporting a partial completion status in the completion record (described in section 3.13). Additionally, page faults on completion record addresses are reported through the Event Log while it is enabled. KMD must ensure that the Event Log is sufficiently large and must process entries in a timely manner, to avoid subsequent Event Log writes from the device being blocked due to log full conditions.

When processing an Event Log entry for a page fault on a completion record, KMD is expected to do the following:

1. If the First Error in Batch flag is 1, discard any previously recorded errors associated with the Batch Identifier. This can happen when a Batch completion is lost because of an Abort command or an internal hardware error. In the usual case, no errors will be recorded, and no action needs to be taken.

2. Attempt to fix the page fault corresponding to the Fault Address and PASID reported in the Event Log entry, and if successful, write the completion record to the Fault Address and generate the completion interrupt, if the Completion Interrupt Required field in the Event Log entry is 1.

3. If there is an error writing the completion record and the completion record is for a descriptor in a batch, KMD associates the error with the Batch Identifier of the Event Log entry and tracks it until the Event Log entry for the corresponding Batch descriptor is observed. KMD does not need to track successfully written completion records.

When processing an Event Log entry for a Batch descriptor with the error code indicating that one or more descriptors in the batch had Event Log entries and the Completion Record Required field in the Event Log entry is 1, KMD does the following before writing the Batch completion record to memory provided an error has been recorded with a matching Batch Identifier:

1. If the Status field of the completion record within the event log entry is 0x01, KMD changes it to 0x05 and changes the Result field to 1.

2. If Status is 0x06, change Result to 1.

3. If Status is any other value, the completion record should not be changed.

4. KMD should then clear the recorded error in preparation for the next batch with the same Batch Identifier.

If no error has been recorded for this batch (all completion records were written successfully), KMD writes the Batch completion record as-is. Software should then generate the completion interrupt, if the Completion Interrupt Required field in the Event Log entry is 1.

When an application or UMD receives a completion record indicating partial completion, it can choose to fix the page fault and resubmit a descriptor to the device to complete the remainder of the operation. In most cases, the original descriptor may need to be updated to adjust the Transfer Size field based on the amount of work already completed. For certain operations, additional updates to the original descriptor may be required. If Batch Continuation Support in GENCAP is 1, when resubmitting a Batch descriptor that was terminated early (as described in section 8.3.2), software can set the Batch Error flag in the descriptor based on the Result field in the completion record for the prior execution of that batch

descriptor. This ensures that the Status and Result fields in the final completion record for the batch reflect the correct status across all the descriptors in the batch.

# 7.4 Software Requirements for Inter-Domain Operations

As described in section 3.13.3, each inter-domain or Update Window operation specifies at least one IDPT handle that references an entry in the Inter-Domain Permissions Table on that device. The IDPT controls the connection and communication between the different address spaces (PASIDs). Some of the key software considerations when using inter-domain or Update Window operations are as follows:

- Privileged software (e.g., KMD) is responsible for discovery and enumeration of inter-domain capabilities, and configuration of the IDPT in each device.

- For an inter-domain or Update Window operation to be successful, an IDPTE must be set up before the descriptor is submitted to the device.

- An IDPT is specific to a device; hence to use a given IDPTE, the owner and submitter processes (described in section 3.13.3) must subscribe to the same physical device.

- The owner and submitter may use the same SWQ or different SWQs/DWQs on the device. The owner does not need to have access to a WQ on the device if it does not need to use the Update Window operation.

- For a type 0 SASS and type1 SAMS IDPTE:

The owner typically requests the KMD through a system call interface (e.g., ioctl) to set up an IDPTE of the appropriate type, with the necessary window parameters.

o It is the responsibility of the owner to communicate the system file handle representing the IDPTE, IDPT handle and window parameters to the submitter. The exact mechanism used to exchange this information depends on the software implementation.

o A software implementation may require each submitter to explicitly register itself with the KMD using a system call interface (e.g., ioctl) prior to first use of an IDPT handle in an inter-domain descriptor. Subsequent uses of the IDPT handle in inter-domain descriptors may be done without involving the KMD.

If the Allow Update field in the IDPTE is 1, the owner can issue an Update Window descriptor to modify window attributes of the IDPTE. It is the responsibility of software to coordinate changes to IDPTE window attributes with any concurrent access to the window by a submitter.

An example software sequence showing the communication between an owner, submitter, KMD, and hardware is shown in Figure 7-1.

![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/87ac176cfba99e3048c162c295d4b4d592afaa5ce918e67dfa5d3b58888052a0.jpg)



Figure 7-1: Example Software Flow for an Inter-Domain Operation


# 7.5 Virtualization Software

Intel DSA is virtualized using the Intel Scalable IOV model, described in the Intel® Scalable I/O Virtualization Architecture Specification. The virtualization software architecture is shown in Figure 7-2. Virtualization of the device is supported by a software component called the Intel DSA Virtual Device Composition Module (VDCM), which composes a virtual Intel DSA device and exposes it to the guest. The VDCM is a VMM specific module and is responsible for communicating with the VMM to facilitate device virtualization. Depending on the host system software architecture, the VDCM may be developed as a user level module, as part of the kernel-mode driver, as a separate kernel module, or as part of the VMM.

The KMD in the Host OS is extended to support the VDCM operations required for virtualization. The KMD with virtualization extensions is called the host driver. The KMD in the Guest OS may run exactly like in a non-virtualization environment or it may be optimized to run in a VM. The KMD in the guest OS is called the guest driver. The host driver controls and manages the physical device and allows sharing of the device among multiple guest drivers. A single Intel DSA driver per OS may be developed to work in the non-virtualized OS, Host OS, and Guest OS.

# 7.5.1 Virtual Intel® DSA Device

The virtual device implemented by the VDCM, called VDEV, emulates the same interface as the physical Intel DSA device, so that the same device driver can run in both the host OS and the guest OS. The guest driver accesses the virtual device through MMIO registers using the same software interface as the physical device. The VDCM emulates the behavior of the virtual device and mediates guest subscription

of the device through the host driver. Control path operations on the VDEV from the VM (e.g., dedicated WQ configuration) are trapped by the VMM and emulated by the VDCM, but fast path operations (descriptor submission and descriptor completion) are directly mapped to the VM.

Within a guest, some features of the device may not be supported. The capability registers indicate to the guest which features are available. For example, the number of work queues or groups available in the virtual device may differ from the number available in the physical device. Another example of a feature that may not be supported is interrupt message storage.

Some aspects of Group and WQ configuration are not modifiable by the guest, indicated by the Configuration Support capability in GENCAP. For example, the size of each WQ must be configured by the host driver before starting the device and may not be changed by a guest that the WQ is subsequently assigned to. To indicate this, the VDCM should always return the value 0 in the Configuration Support field in the GENCAP register of the VDEV.

If a WQ is assigned to multiple guests, it is configured as a Shared WQ by the host driver. None of the WQ configuration registers for such a WQ can be changed by the guest driver. This is indicated to the guest by the value 0 in the WQ Mode Support field of the WQCFG register.

If a WQ is assigned to a single guest, the guest driver may decide whether it is to be a Dedicated WQ or a Shared WQ. In this case, the guest driver may also configure the WQ Threshold, Priv, PASID Enable, and PASID. This is indicated to the guest by the value 1 in the WQ Mode Support field of the WQCFG register. See Table 9-7 for details of WQ configuration support.

![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/5ba44b8817e1a05461132e6a96755df9d3832b8fec80986827614eeb795745db.jpg)



Figure 7-2: Intel® Scalable IOV for Intel® DSA


# 7.5.2 Portal Virtualization

For each WQ included in a VDEV, the VDCM directly maps some of the WQ's physical portals into the VM. For a WQ shared by multiple guests, the host driver retains control of the unlimited portal, and the VDCM maps only the limited portal into the guest. When a guest submits a descriptor using its unlimited portal address (after the guest's limited portal has returned Retry), the VMM traps on the portal write and the host driver submits the descriptor using the physical unlimited portal to provide forward progress to the guest. If the physical unlimited portal also returns Retry, the host driver may use the same approaches described in section 7.1.

For a WQ assigned to a single guest, the VDCM should map both the limited portal and the unlimited portal. That way, if the guest driver chooses to configure the WQ as a Shared WQ, it can set the WQ Threshold and manage forward progress assurance on the WQ itself by mapping the limited portal directly into its user-mode clients and using the unlimited portal for kernel-mode operations.

Figure 7-2 shows that VDCM has created VDEV1 for Guest 1 with one shared WQ (SWQ) and VDEV2 for Guest 2 with one SWQ and one dedicated WQ (DWQ). Guest 1 and Guest 2 share the same SWQ in the device. The DWQ can be assigned to only one VM. The corresponding SWQ and DWQ portals are directly mapped into the respective VMs for fast path operations. For the SWQ, the same limited portal is mapped into both VMs.

The VDCM maps only IMS portals into the guest. The MSI-X portals are reserved for host use. If the virtual device visible to the guest does not report support for IMS, the IMS portals are mapped into the guest's virtual BAR2 in place of the MSI-X portals and the dummy portal (described in section 9.2.21) may be mapped into the address ranges corresponding to the guest's virtual IMS portals. See section 7.5.4 for a description of interrupt virtualization.

# 7.5.3 SVM and PASID Virtualization

When a virtual Intel DSA device is assigned to a VM, all WQs used by the VM must be configured to use PASID. The VMM allocates a default Host PASID for the VM and configures the PASID table entry for that PASID in the IOMMU for second level address translation (GPA  $\rightarrow$  HPA). This PASID is used when the guest configures a virtual WQ in dedicated mode with PASID disabled. For the guest to use the virtual device in this way, the VDEV need not support the PASID, ATS, and PRS PCIe capabilities (even though these capabilities are enabled in the physical Intel DSA device).

To support SVM in the guest, the VDEV includes support for the ATS, PASID, and PRS capabilities, and the VMM exposes a virtual IOMMU to the guest. The guest OS sets up PASID table entries in the virtual IOMMU's PASID table. Since guest software uses Guest PASIDs and the physical device uses Host PASIDs, the VMM must manage Guest PASID to Host PASID mapping.

Some VMMs may choose to use a para-virtualized or enlightened virtual IOMMU where the guest doesn't generate its own Guest PASIDs but instead requests Guest PASIDs from the virtual IOMMU. In this case, the VMM may use the same value for the Guest PASID as for the Host PASID for each requested Guest PASID, simplifying PASID management in the VMM. Otherwise, the guest OS allocates its own Guest PASIDs for its SVM operations and the VMM must allocate a Host PASID for each Guest PASID.

The method for setting up a Guest PASID to Host PASID mapping depends on whether the WQ is in dedicated or shared mode. If a WQ is assigned to a single VM, the guest driver can decide whether to configure it as a DWQ or as an SWQ to be shared across multiple applications within the VM. If a WQ is assigned to multiple VMs, then it is configured as an SWQ by the host driver and the guest cannot change the WQ Mode.

When the guest driver enables a WQ in dedicated mode with the WQ PASID Enable field in the VDEV equal to 1, the VMM creates a mapping for the Guest PASID in the WQ PASID field. If the WQ PASID Enable field is 0, the VMM uses the VM's default Host PASID. In either case, the host driver writes the proper Host PASID to the WQ PASID field of physical WQCFG register and writes 1 to the WQ PASID Enable field.

If a WQ is configured in shared mode, by either the host driver or the guest driver, the VMM enables the PASID Translation VMX execution control in the VMCS (VM Control Structure). The guest uses the ENQCMD or ENQCMDS instructions to submit descriptors. On the first submission for a Guest PASID, ENQCMD/S causes a VM exit since the PASID translation table doesn't have a mapping for the Guest PASID, and the VMM creates a mapping for it.

To create a mapping for a Guest PASID, the VMM looks at the PASID table entry for the Guest PASID in the virtual IOMMU's PASID table. If the Guest PASID is configured for first-level translation in the virtual IOMMU, the VMM allocates a new Host PASID, configures its PASID table entry for nested first-level (GVA to GPA) and second-level (GPA to HPA) translations, and sets up the VMCS PASID translation table to map the Guest PASID to the Host PASID. If the Guest PASID is not configured in the virtual IOMMU, the VMM sets up the VMCS PASID translation table to map the Guest PASID to the VM's default Host PASID, which is already configured in the physical IOMMU.

# 7.5.4 Interrupt Virtualization

The VDCM virtualizes interrupts by exposing a virtual MSI-X capability in the VDEV. The Interrupt Message Storage Size field in GENCAP in the VDEV may be 0. The VDCM requests that the host driver allocate an entry in the Interrupt Message Storage for each interrupt available to the VM. The VDCM maps the Limited IMS Portal for each WQ into the VM at the offset of both the Unlimited MSI-X Portal and the Limited MSI-X Portal. When the guest uses its MSI-X portal address to submit descriptors, it is actually using the physical IMS portal, so that guest interrupts are always generated using the IMS.

When the guest OS configures a virtual MSI-X entry, the VDCM or the host driver requests that the Host OS or VMM allocate a physical interrupt and program it into the IOMMU's interrupt posting structure using the vector and VCPU information from the virtual MSI-X table entry. The Host OS or VMM passes the physical interrupt address and data value to the host driver, which is responsible for configuring the physical interrupt into the allocated Interrupt Message Storage entry, including setting the IMS PASID field to the PASID of the guest.

The Command Capabilities register in the VDEV indicates support for the Request Interrupt Handle command, requiring the guest to use the Request Interrupt Handle command to obtain an interrupt handle associated with each MSI-X table entry. The VDCM responds to the command with the index in the IMS corresponding to the virtual MSI-X table entry. The guest places the interrupt handle in each descriptor that requests an interrupt. The physical device uses the handle to identify the Interrupt Message Storage entry to be used to generate the completion interrupt. It checks the PASID of the

descriptor against the PASID field in the IMS entry. If a guest requests an interrupt using an interrupt handle that has not been assigned to it, the PASID won't match, so the interrupt will not be generated.

When migrating a VM or resuming a VM after it has been suspended, interrupt handles that were allocated to the VM may no longer be available. To inform the guest that one or more interrupt handles have been revoked, the VDCM sets the Interrupt Handles Revoked bit in the virtual INTCAUSE register and generates an interrupt to the guest, using MSI-X entry 0 in the VDEV. The guest clears the Interrupt Handles Revoked bit and then uses the Request Interrupt Handle command to obtain new handles for any MSI-X and/or IMS entries that are in use. After ensuring that all threads have stopped using the revoked handles, the guest submits a Drain descriptor using each new interrupt handle. The Drain waits for completion of any descriptors that were submitted using the revoked handle; these descriptors complete with Operation Status 0x19, Invalid Interrupt Handle. Upon completion of the Drain descriptor, Intel DSA hardware generates the expected completion interrupt for these descriptors, so that the errors in the completion records are recognized by software and the descriptors can be resubmitted using the new handle. See Figure 7-2 for pseudocode of the steps to be performed in the guest.

When the guest writes the Ignore or Mask bits of the virtual MSI-X table, the VDCM writes the corresponding IMS table entry. When the guest reads the virtual MSI-X Pending bit array, the VDCM constructs the value from the values of the Pending bits of the IMS table entries assigned to that guest.

The VDCM should provide one additional MSI-X table entry, used for errors and command completions. The VDCM itself is responsible for generating virtual interrupts for these events using the vector and VCPU information in the virtual MSI-X table entry.

If the Interrupt Message Storage Support capability in the VDEV is 1, the IMS is virtualized in much the same way as MSI-X.

```verilog
Submitter Thread(s) using intrtable entry idx Interrupt Handle Revocation Handler atomic_inc(intr_handle_users(idx)) Clear INTCAUSE.Interrupt_Handles_Revoked // Check for revoked interrupt handle for each idx in MSI-X table and IMS table { new_intr_handle  $\equiv$  dsa_request_intr_handle(idx) atomic_dec(intr_handle_users(idx)) if (new_intr_handle  $= =$  intr_handle(idx)) // Wait for new handle to be available. continue // Interrupt handle did not change intr_handle  $(idx) =$  REVOKED yield() // Wait for submitters to complete submission // of any descriptors using the revoked handle. while (intr_handle_users  $(idx)\neq 0)$  yield( ) intr_handle  $(idx) =$  new_intr_handle enqcmd(dsa_desc) or movdir64b(dsa_desc) Drain Descriptor(new_intr_handle) }
```

Figure 7-3: Guest Steps to Handle Interrupt Handle Revocation

When a guest is destroyed, after its PASIDs are drained, the PASID Enable field should be cleared in all the IMS entries allocated to the guest, to ensure that those entries cannot be improperly used by another guest when the PASIDs are reassigned.

# 7.5.5 Capability Virtualization

Intel DSA exposes its capabilities to software via capability registers, described in section 9.2. This enables VDCM to expose a subset of device capabilities to the VM through the virtual device's capability registers, allowing the virtual device to be compatible with multiple generations of devices. This capability virtualization enables a VM image with a guest driver to be started on or migrated to physical machines containing different generations of Intel DSA devices. This allows creation of pools of compatible physical machines in a datacenter where the same VM image can be started or migrated.

# 7.5.6 Virtualization of Inter-Domain Features

If the PASID capability is not enabled for a guest, the VDCM does not expose inter-domain capabilities to that guest. If the PASID capability is enabled for a guest, the VDCM virtualizes inter-domain operations by exposing a virtual IDPT capability in the VDEV. The VM allocates entries in the virtual IDPT and configures them using MMIO writes to the corresponding virtual IDPTE. The VDCM translates Guest PASID values in the virtual IDPTE into the corresponding Host PASID values and requests the host driver to allocate a physical IDPTE for each virtual IDPTE allocated by the VM. The Request IDPT Handle field in CMDCAP of the VDEV is 1, indicating that the guest must use the Request IDPT Handle command to obtain the IDPT handle associated with each IDPTE in the VDEV. The VDCM responds to the command with the index of the physical IDPTE corresponding to the virtual IDPTE. When a guest submits descriptors with the IDPT handle, it is actually using the physical IDPTE containing the Host PASID values associated with that guest. The guest KMD can maintain an association of the handle with the virtual IDPTE.

When migrating a VM or resuming a VM after it has been suspended, IDPT handles that were allocated to the VM may no longer be available. An inter-domain or Update Window descriptor submitted by the VM that references an unavailable IDPT handle completes with an error code as described in section 5.8.1. Upon receiving a completion with an invalid IDPT handle error, the application in the VM uses an appropriate system call interface (e.g., ioctl) to request a new IDPT handle from the guest KMD. The guest KMD uses the Request IDPT Handle command to obtain a new IDPT handle as mentioned above. The VDCM allocates a new entry in the physical IDPT and initializes the entry appropriately based on the values in the corresponding virtual IDPTE. The VDCM responds to the command with the index of the physical IDPTE as the new handle to use. The guest KMD provides the new handle to the application and may record the new handle as the one associated with the virtual IDPTE.

The VDCM may map the region of memory corresponding to the virtual IDPT into the VM using read-only mapping. When a guest writes to a virtual IDPTE, the VDCM writes the corresponding location in the DRAM backed region for the virtual IDPT and requests the host driver to write the corresponding values to the physical IDPTE. When the guest reads the virtual IDPTE, it reads the values directly from the DRAM region without a VM exit.

If the VDCM exposes a type 1 SAMS IDPTE to a guest, it must virtualize the submitter bitmap associated with that IDPTE. The VM allocates memory for the virtual bitmap and configures the virtual IDPTE with

the bitmap address. The VDCM allocates memory for a corresponding physical bitmap and updates the physical IDPTE. The guest can read or write to the virtual bitmap region without a VM exit. The Invalidate Submitter Bitmap Cache capability in the VDEV is 1, indicating that the guest must use the Invalidate Submitter Bitmap Cache command after any changes to the bitmap to ensure that the updated values are used by the VDEV. When the VM issues an Invalidate Submitter Bitmap Cache command specifying the portion of the bitmap that has been updated, the VDCM updates the corresponding bits in the physical bitmap. If the guest fails to issue an Invalidate Submitter Bitmap Cache command, the physical bitmap will not be updated by the VDCM, and any subsequent descriptors referencing that IDPTE may not complete as intended.

When a guest is destroyed, after its PASIDs are drained, any IDPTEs assigned to that VM must be marked unusable to ensure that those entries cannot be improperly used by another guest when the PASIDs are reassigned.

# 7.5.7 State Migration During VM Migration

Intel DSA virtualization supports live migration of VMs. During the final phase of live VM migration, the VMM suspends the VM and then issues a suspend command to all the virtual devices of the VM and waits for suspend to complete. The VMM then saves the virtual device state, migrates it along with the rest of the VM state, and restores the virtual device state on the destination machine.

To suspend the virtual Intel DSA device, the VDCM requests that the host driver drain all the Host PASIDs assigned to the VM. The host driver issues a Drain PASID command for each assigned PASID or it may issue Drain All if a large number of PASIDs are assigned to the VM. After completion of the Drain commands, the virtual device reaches the suspended state. If there are pending interrupts for the VM in the interrupt posting structure of the IOMMU, they are delivered to the virtual APIC. The virtual device state is transferred to the destination machine along with the rest of the state of the VM.

On the destination machine, the VMM creates a new virtual Intel DSA device for the VM and restores the virtual device state to it. Specifically, it configures IMS entries for interrupts that are configured in the virtual MSI-X table, assigns physical WQs to the VM according to the virtual device configuration, and sets up the physical IOMMU for DMA remapping and interrupt remapping/posting. For a Dedicated WQ, the destination DWQ must be the same or larger size compared to the original DWQ since the guest driver may continue to use the old DWQ size. The capability virtualization described in section 3.16 ensures that the virtual device can work on multiple generations of Intel DSA devices.

See section 7.5.4 for a description of interrupt handle revocation after VM migration. See section 7.5.6 for a description of IDPT handle revocation after VM migration.

# 7.5.8 Virtualization of Event Log

The Event Log capability may be exposed to a guest independently of whether Event Log is enabled in hardware. When the Event Log capability is exposed to a guest, the guest may or may not enable the feature. If a guest does not enable Event Log, the VDCM must use the virtual SWERROR to report errors to the guest. If multiple errors are present to be injected into the guest, the VDCM may set the Overflow field in the virtual SWERROR register or it may report errors to the guest one at a time, waiting for the guest to clear each error in the virtual SWERROR before the VDCM reports the next one.

If a guest enables the Event Log, the VDCM must use it to report errors to the guest instead of using SWERROR. The remainder of this section covers the case when Event Log is enabled by both the host and the guest.

When a guest enables the Event Log in the virtual Intel DSA device, it allocates space for its event log in its own memory and writes the base address and size to the EVLCFG fields in the virtual Intel DSA device.

While the host driver is processing the Event Log, it determines for each event log entry whether it is specific to a guest. The VDCM translates the PASID, WQ Index, and portal identifier from the host event log entry to guest values, writes the modified event log entry into the guest event log in guest memory, updates the Event Log Tail field in the virtual EVLSTATUS register, and then generates an interrupt into the guest if required. This does not require a VM exit from the guest if the host event log processing is performed on a separate thread.

When the guest receives an event log interrupt, it reads the Event Log entries from its own memory. No VM exits are required to read the Event Log, but VM exits may be required to read Event Log Tail, write Event Log Head, and clear Interrupt Pending in the virtual EVLSTATUS register.

When writing event log entries into guest memory, the VDCM performs address translation using the fields in the virtual EVLCFG register and the translation tables configured in the guest's virtual IOMMU.

- If there is no virtual IOMMU in the guest or it is disabled, the Event Log Base Address in the virtua EVLCFG register is a guest physical address. The PASID Enable, PASID, and Priv fields in the virtual EVLCFG are ignored.

- If the virtual IOMMU is in legacy mode, then the Guest PASID Enable must be 0 in the virtual EVLCFG. Otherwise, writing to the guest event log causes a fault in the virtual IOMMU. (Refer to the Intel Virtualization Technology for Directed I/O Specification for details on the error code reported.)

If the virtual IOMMU is in scalable mode,

If the Guest PASID Enable is 0, translation is performed using RID_PASID in the virtual IOMMU context entry.

If the Guest PASID Enable is 1, translation is performed using the PASID and Priv fields in the virtual EVLCFG register.

# Handling Guest Event Log Full Condition

As described in section 5.9, when the physical hardware needs to write to the Event Log and the Event Log is full, hardware waits until software writes to the Event Log Head to make space available in the event log. This delays completion of any descriptors that cause a write to the event log and may also delay execution of subsequent descriptors.

As described in section 9.2.2, the Event Log Overflow Support field in GENCAP indicates whether the device drops new events if the Event Log is full or if it blocks until software has updated the Event Log Head.

A VDCM may report the Event Log Overflow Support field in the virtual GENCAP register as 0 and mimic the blocking behavior of the physical hardware. In this case, to avoid impacting other uses of the

device, the host driver should continue to process the physical event log even if a guest's event log is full. As the host driver processes the physical event log and forwards entries into the appropriate guests' event logs, if a guest event log is full, the VDCM stores that guest's event log entries in an internal VDCM Event Queue associated with the guest. As the guest frees entries in the event log by moving the head, VDCM transfers event log entries from the VDCM Event Queue into the guest event log.

Alternatively, a VDCM may configure the virtual hardware to report Event Log Overflow Support in the virtual GENCAP register as 1. In this case if the virtual Event Log is full, the VDCM may discard events pertaining to that virtual event log and log an error indicating the Event Log full condition in the virtual SWERROR register. If an Event Log full error is logged in SWERROR, or if SWERROR reports an overflow while the Event Log is full, guest software must assume that one or more events may have been dropped and take appropriate steps to notify and/or terminate any impacted applications as needed. A VDCM may also choose to implement a fixed size VDCM Event Queue per guest to log events when the virtual Event Log is full and begin discarding events for a guest only if the corresponding Event Queue becomes full.

If the VDCM Event Queue is of a fixed size, the VDCM can size it appropriately to make an overflow situation less likely (based on descriptors in flight, with no additional descriptors being submitted), by setting the queue size as the sum of the following:

Number of WQs  $\times$  WQ size  $\times$  (Maximum Batch Size + 1).

Number of Engines  $\times$  Maximum Batch Descriptors in Progress  $\times$  (Maximum Batch Size + 1).

Number of Engines  $\times$  Maximum Work Descriptors in Progress.

For example, if a guest is assigned the following:

1WQ in a group with 1 engine.

WQ Size = 16.

WQ Maximum Batch Size = 32.

VDCM Event Queue Size  $= 1 \times 16 \times (32 + 1) + 1 \times 16 \times (32 + 1) + 1 \times 128 = 1184$  entries or about 76 KB.

# 8 Descriptor Formats

# 8.1 Common Descriptor Fields

Intel DSA descriptors are 64 bytes. Some descriptor fields are common to all operation types and some fields are dependent on the operation type. This section describes the fields that are common to more than one operation type. The diagram for each operation type indicates which of the common fields are used for that operation type and what the operation-specific fields are.

Common fields include both trusted fields and untrusted fields. Trusted fields are always trusted by the device since they are populated by the CPU or by privileged (ring 0 or VMM) software on the host. The untrusted fields are directly supplied by client software.


Generic Descriptor Format


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td rowspan="3" colspan="10">Operation-specific fields</td><td>40</td></tr><tr><td>48</td></tr><tr><td>56</td></tr></table>

# 8.1.1 Trusted Fields

# Offset: 0; Size: 4 bytes (32 bits)

When a descriptor is submitted to an SWQ, these fields carry the Privilege and PASID of the software entity that submitted the descriptor. When a descriptor is submitted to a DWQ, these fields in the descriptor are ignored; the device uses the WQ Priv and WQ PASID fields of the WQCFG register.

On Intel CPUs, when software submits a descriptor to an SWQ using ENQCMD, these fields in the source descriptor are reserved. The value of IA32_PASID MSR is placed in the PASID field and the Priv field is set to 0 before the descriptor is sent to the device. When software uses ENQCMDS, these fields in the source descriptor must be initialized appropriately by software. If the Privileged Mode Enable field of the PCI Express PASID capability is 0, the Priv field must be 0.

These fields are ignored for any descriptor in a batch. The corresponding fields of the Batch descriptor are used for every descriptor in the batch.

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>31</td><td>Priv (User/Supervisor)
0: The descriptor is a user-mode descriptor submitted directly by a user-mode client or submitted by the kernel on behalf of a user-mode client.
1: The descriptor is a kernel-mode descriptor submitted by kernel-mode software.</td></tr><tr><td>30:20</td><td>Reserved</td></tr><tr><td>19:0</td><td>PASID
This field contains the Process Address Space ID of the requesting process and indicates the default address space for this descriptor.</td></tr></table>

Table 8-1: Descriptor Trusted Fields

# 8.1.2 Operation

# Offset: 7; Size: 1 byte (8 bits)

This field specifies the operation to be executed.

<table><tr><td>0x00</td><td>No-op</td></tr><tr><td>0x01</td><td>Batch</td></tr><tr><td>0x02</td><td>Drain</td></tr><tr><td>0x03</td><td>Memory Move</td></tr><tr><td>0x04</td><td>Fill</td></tr><tr><td>0x05</td><td>Compare</td></tr><tr><td>0x06</td><td>Compare Pattern</td></tr><tr><td>0x07</td><td>Create Delta Record</td></tr><tr><td>0x08</td><td>Apply Delta Record</td></tr><tr><td>0x09</td><td>Memory Copy with Dualcast</td></tr><tr><td>0x0A</td><td>Translation Fetch</td></tr><tr><td>0x0B - 0xOF</td><td>Reserved</td></tr><tr><td>0x10</td><td>CRC Generation</td></tr><tr><td>0x11</td><td>Copy with CRC Generation</td></tr><tr><td>0x12</td><td>DIF Check</td></tr><tr><td>0x13</td><td>DIF Insert</td></tr><tr><td>0x14</td><td>DIF Strip</td></tr><tr><td>0x15</td><td>DIF Update</td></tr><tr><td>0x16</td><td>Reserved</td></tr><tr><td>0x17</td><td>DIX Generate</td></tr><tr><td>0x18</td><td>Type Conversion</td></tr><tr><td>0x19</td><td>Reduce</td></tr><tr><td>0x1A</td><td>Reduce with Dualcast</td></tr><tr><td>0x1B</td><td>Gather Reduce</td></tr><tr><td>0x1C</td><td>Gather Copy</td></tr><tr><td>0x1D</td><td>Scatter Copy</td></tr><tr><td>0x1E</td><td>Scatter Fill</td></tr><tr><td>0x1F</td><td>Reserved</td></tr><tr><td>0x20</td><td>Cache flush</td></tr><tr><td>0x21</td><td>Update Window</td></tr><tr><td>0x22</td><td>Reserved</td></tr><tr><td>0x23</td><td>Inter-Domain Copy</td></tr><tr><td>0x24</td><td>Inter-Domain Fill</td></tr><tr><td>0x25</td><td>Inter-Domain Compare</td></tr><tr><td>0x26</td><td>Inter-Domain Compare Pattern</td></tr><tr><td>0x27 - 0xFF</td><td>Reserved</td></tr></table>

Table 8-2: Operation Types

# 8.1.3 Flags

Offset: 4; Size: 3 bytes (24 bits)

<table><tr><td>Bits</td><td colspan="2">Description</td></tr><tr><td>23:16</td><td colspan="2">Operation-Specific FlagsSee the description of each operation type for the usage of this field.This field is reserved for all operation types where no meaning is given.</td></tr><tr><td>15</td><td colspan="2">Reserved. Must be 0 .</td></tr><tr><td>14</td><td colspan="2">Cache Control 2This field together with Cache Control 1 and Cache Control 3 specify a hint to direct the placement of data writes in the memory hierarchy as described in section 8.1.3.1.This field is reserved for operation types that do not write to memory. Certain combinations of the cache control flags are reserved as described in section 8.1.3.1.</td></tr><tr><td>13</td><td colspan="2">Strict Ordering0: Default behavior: writes to the destination can become globally observable out of order.The completion record write has strict ordering, so it always completes after all writes to the destination are globally observable.1: Forces strict ordering of all memory writes produced by the device and ensures that they become globally observable in that order.This field is reserved for operation types that do not write to memory.If the Enable Relaxed Ordering field in the PCIe config Device Control register is 0 , this field is ignored, and all memory writes use strict ordering.Note that this flag has nothing to do with the order in which descriptors are executed. It only affects ordering of the writes generated by this descriptor.</td></tr><tr><td>12</td><td colspan="2">Completion Record TC SelectorThis field selects the Traffic Class value used for writing the completion record. It selects one of the two TC values in the Group Configuration Register corresponding to the WQ that the descriptor was submitted to. See section 4.2 for information on the use of Traffic Classes.0: Use TC-A in the Group Configuration Register.1: Use TC-B in the Group Configuration Register.This field is reserved when Completion Record Address Valid is 0 .</td></tr><tr><td rowspan="12">11</td><td colspan="2">Address 3 TC Selector
This field selects one of the two Traffic Class values in the Group Configuration Register corresponding to the WQ that the descriptor was submitted to.
0: Use TC-A in the Group Configuration Register.
1: Use TC-B in the Group Configuration Register.
This field selects the TC value for memory accesses for the operations shown in the table.
This field is reserved for all other operation types.</td></tr><tr><td>Operation</td><td>Address field for which this selector controls the TC</td></tr><tr><td>Memory Copy with Dualcast</td><td>Destination2 Address</td></tr><tr><td>CRC Generation</td><td>CRC Seed.</td></tr><tr><td>Copy with CRC Generation</td><td>Reserved if Read CRC Seed is 0.</td></tr><tr><td>Create Delta Record</td><td>Delta Record Address</td></tr><tr><td>Reduce</td><td>Destination Address</td></tr><tr><td>Reduce with Dualcast</td><td>Destination1 Address</td></tr><tr><td>Gather Reduce</td><td>Scatter-Gather List Address</td></tr><tr><td>Gather Copy</td><td>Source SGL Address</td></tr><tr><td>Scatter Copy</td><td>Destination SGL Address</td></tr><tr><td>Scatter Fill</td><td></td></tr><tr><td rowspan="9">10</td><td colspan="2">Address 2 TC Selector
This field selects one of the two Traffic Class values in the Group Configuration Register corresponding to the WQ that the descriptor was submitted to.
0: Use TC-A in the Group Configuration Register.
1: Use TC-B in the Group Configuration Register.
This field selects the TC value for memory accesses for the operations shown in the table.
This field is reserved for all other operation types.</td></tr><tr><td>Operation</td><td>Address field for which this selector controls the TC</td></tr><tr><td>Operations that contain Destination Address</td><td>Destination Address</td></tr><tr><td>Drain</td><td>Readback Address 2</td></tr><tr><td>Memory Copy with Dualcast</td><td>Destination1 Address</td></tr><tr><td>Compare Create Delta Record Reduce</td><td>Source2 Address</td></tr><tr><td>Reduce with Dualcast</td><td></td></tr><tr><td>Scatter Copy</td><td>Base Address</td></tr><tr><td>Scatter Fill</td><td></td></tr><tr><td rowspan="8">9</td><td colspan="2">Address 1 TC Selector
This field selects one of the two Traffic Class values in the Group Configuration Register corresponding to the WQ that the descriptor was submitted to.
0: Use TC-A in the Group Configuration Register.
1: Use TC-B in the Group Configuration Register.
This field selects the TC value for memory accesses for the operations shown in the table.
This field is reserved for all other operation types.</td></tr><tr><td>Operation</td><td colspan="1">Address field for which this selector controls the TC</td></tr><tr><td>Operations that contain Source Address</td><td colspan="1">Source Address</td></tr><tr><td>Batch</td><td colspan="1">Descriptor List Address</td></tr><tr><td>Drain</td><td colspan="1">Readback Address 1</td></tr><tr><td>Compare
Create Delta Record
Reduce
Reduce with Dualcast</td><td colspan="1">Source1 Address</td></tr><tr><td>Apply Delta Record</td><td colspan="1">Delta Record Address</td></tr><tr><td>Gather Reduce
Gather Copy</td><td colspan="1">Base Address</td></tr><tr><td>8</td><td colspan="2">Cache Control 1
This field together with Cache Control 2 and Cache Control 3 specify a hint to direct the placement of data writes in the memory hierarchy as described in section 8.1.3.1.
This field is reserved for operation types that do not write to memory. Certain combinations of the cache control flags are reserved as described in section 8.1.3.1.</td></tr><tr><td>7</td><td colspan="2">Check Result
0: Result of operation does not affect the Status field of the completion record.
1: Result of operation affects the Status field of the completion record, if the operation is successful. Status is set to either Success or Success with false predicate, depending on the result of the operation. See the description of each operation for the possible results and how they affect the Status.
This field is used for Compare, Compare Pattern, Create Delta Record, Type Conversion, Reduce, Reduce with Dualcast, and Gather Reduce operations. It is reserved for all other operation types.</td></tr><tr><td>6</td><td colspan="2">Reserved. Must be 0.</td></tr><tr><td>5</td><td colspan="2">Cache Control 3
This field together with Cache Control 1 and Cache Control 2 specify a hint to direct the placement of data writes in the memory hierarchy as described in section 8.1.3.1.
This field is reserved for operation types that do not write to memory. Certain combinations of the cache control flags are reserved as described in section 8.1.3.1.</td></tr><tr><td>4</td><td colspan="2">Request Completion Interrupt
0: No interrupt is generated when the operation completes.
1: An interrupt is generated when the operation completes.
If both a completion record and a completion interrupt are generated, the interrupt is always generated after the completion record is written.
See section 3.7 for information regarding the interrupt to be generated.
This field is reserved if User-mode Interrupts Enable is 0 and Priv is 0 (indicating a user-mode descriptor). If WQ PASID Enable control is 0, this field is not-reserved, independent of the setting of the User-mode Interrupts Enable control (see section 9.2.8).</td></tr><tr><td>3</td><td colspan="2">Request Completion Record
0: A completion record is written only if the operation status is not equal to 0x01, 0x02, or 0x05.
1: A completion record is always written at the completion of the operation.
This flag must be 1 for any operation that yields a result, such as Compare.
This flag must be 0 if Completion Record Address Valid is 0.</td></tr><tr><td>2</td><td colspan="2">Completion Record Address Valid
0: The completion record address is not valid.
1: The completion record address is valid.
This flag must be 1 for any operation that yields a result, such as Compare. It should be 1 for any operation that uses virtual addresses, because of the possibility of a page fault, which must be reported via the completion record. For best results, this flag should be 1 in all descriptors, because it allows the device to report errors to the software that submitted the descriptor. If this flag is 0 and an unexpected error occurs, the error is reported in the event log, if enabled, or in the SWERROR register, and the software that submitted the request may not be notified of the error.
Notwithstanding the above caveats, if the descriptor uses physical addresses or uses virtual addresses that software guarantees are present (pinned), and software has no need to receive notification of any other types of errors, this flag may be 0.</td></tr><tr><td>1</td><td colspan="2">Block On Fault
0: Page faults cause partial completion of the descriptor.
1: The device waits for page faults to be resolved and then continues the operation.
This flag does not affect the handling of page faults on Completion Record Address, Descriptor List Address, or Drain Readback Address, all of which always block on fault. See section 3.13.
This field is reserved if the Block on Fault Enable field in WQCFG is 0.
This field is reserved for certain operation types: No-op, Drain, and Batch.</td></tr><tr><td>0</td><td colspan="2">Fence
0: This descriptor may be executed in parallel with other descriptors in the batch.
1: The device waits for previous descriptors in the same batch to complete before beginning work on this descriptor. If any previous descriptor completed with Status not equal to Success, this descriptor and all subsequent descriptors in the batch are abandoned.
This field may only be set in descriptors that are in a batch. It is reserved in descriptors submitted directly to a Work Queue.</td></tr></table>

Table 8-3: Descriptor Flags

# 8.1.3.1 Cache Control Hints

This section describes the hint implied by each combination of the cache control flags. The cache control flags are Cache Control 1, Cache Control 2, and Cache Control 3. These flags apply to operations that write to memory. Certain encodings are also allowed for the Cache Flush operation as shown in the table below.

For table entries that specify destination readback in the hint, a read of the final destination address is performed before the operation is completed, but after all writes to the destination have been issued by the device. The readback is performed only if the descriptor is completed successfully.

Some implementations may not detect an error for reserved encodings. Software should not rely on the behavior of reserved encodings. Additional information pertaining to write durability and cache control flags is in sections 3.10 and 3.11.

<table><tr><td colspan="3">Cache Control 
flags</td><td rowspan="2">Description</td></tr><tr><td>3</td><td>2¹</td><td>1</td></tr><tr><td>0</td><td>0</td><td>0</td><td>Write data to memory
If a write operation targets a cache line that is present in the cache hierarchy, it may be evicted from the cache. This encoding is also supported for the Cache Flush operation.</td></tr><tr><td>0</td><td>0</td><td>1</td><td>Write data to cache
If a write operation targets a cache line that is present in the cache hierarchy, it may be updated with new data. If the line is not present in the cache hierarchy, a new cache entry may be allocated to contain data written by the descriptor. This encoding is reserved if the Cache Control field in GENCAP is 0.</td></tr><tr><td>0</td><td>1</td><td>0</td><td>Write data to Durable Memory with Destination Readback
Writes to the destination are identified as writes to durable memory. This encoding supports writing to durable memory even when the Durable Write Support field in GENCAP is 0. A destination readback is also performed prior to writing a completion record as described in section 3.9. This encoding is also supported for the Cache Flush operation.
This encoding is reserved if the Destination Readback Support field in GENCAP is 0.</td></tr><tr><td>0</td><td>1</td><td>1</td><td>Write data to cache with Destination Readback
If a write operation targets a cache line that is present in the cache hierarchy, it may be updated with new data. If the line is not present in the cache hierarchy, a new cache entry may be allocated to contain data written by the descriptor. A destination readback is also performed prior to writing a completion record as described in section 3.9.
This encoding is reserved if either the Cache Control or Destination Readback Support field in GENCAP is 0.</td></tr><tr><td>1</td><td>0</td><td>0</td><td>Write data to Durable Memory
Writes to the destination are identified as writes to durable memory.
This encoding is reserved if the Durable Write Support field in GENCAP is 0.</td></tr><tr><td>1</td><td>0</td><td>1</td><td>Reserved</td></tr><tr><td>1</td><td>1</td><td>0</td><td>Write data to memory with Destination Readback
If a write operation targets a cache line that is present in the cache hierarchy, it may be evicted from the cache. A destination readback is also performed prior to writing a completion record as described in section 3.9.
This encoding is reserved if the Destination Readback Support field in GENCAP is 0.</td></tr><tr><td>1</td><td>1</td><td>1</td><td>Reserved</td></tr></table>

Table 8-4: Cache Control Flags

# 8.1.4 Completion Record Address

# Offset 8; Size 8 bytes (64 bits)

This field specifies the address of the completion record. The completion record is 32 bytes and must be aligned on a 32-byte boundary. If the Completion Record Address Valid flag is 0, this field is reserved.

If the Request Completion Record flag is 1, a completion record is written to this address at the completion of the operation. If Request Completion Record is 0, a completion record is written to this address only if there is a page fault or error.

For any operation that yields a result, such as Compare, the Completion Record Address Valid and Request Completion Record flags must both be 1 and the Completion Record Address must be valid.

For any operation that uses virtual addresses, the Completion Record Address should be valid, whether or not the Request Completion Record flag is set, so that a completion record may be written in case there is a page fault or error.

For best results, this field should be valid in all descriptors, because it allows the device to report errors to the software that submitted the descriptor. Otherwise, if an unexpected error occurs, the error is reported in the SWERROR register or event log, and the software that submitted the request may not be notified of the error.

# 8.1.5 Source Address

# Offset: 16; Size: 8 bytes (64 bits)

For operations that read data from memory, this field specifies the address of the source data. There is no alignment requirement for the source address for most operation types. Exceptions are noted in the operation descriptions. If the Source Address and Transfer Size are not both aligned to a multiple of 64 bytes, an implementation may read more source data than required by the descriptor. For example, source data may be read in aligned 32-byte chunks. The excess data is discarded.

# 8.1.6 Destination Address

# Offset: 24; Size: 8 bytes (64 bits)

For operations that write data to memory, this field specifies the address of the destination buffer. There is no alignment requirement for the destination address for most operation types. Exceptions are noted in the operation descriptions.

For some operation types, this field is used as the address of a second source buffer.

# 8.1.7 Transfer Size

# Offset: 32; Size: 4 bytes (32 bits)

This field indicates the number of bytes to be read from the source address to perform the operation.

The maximum allowed transfer size is dependent on the WQ that the descriptor was submitted to. It is specified by the WQ Maximum Transfer Size field for the WQ in the WQ Configuration Table (which is, in turn, limited by the Maximum Supported Transfer Size field in the General Capabilities Register). The Create Delta Record operation has an additional limitation on the maximum allowed transfer size, noted in the description of that operation.

For a Batch operation, this field contains the Descriptor Count. Descriptor Count must be greater than 1. The maximum allowed descriptor count is specified by the WQ Maximum Batch Size field for the WQ in the WQ Configuration Table (which is, in turn, limited by the Maximum Supported Batch Size field in the General Capabilities Register).

Transfer Size must not be 0. For most operation types, there is no alignment requirement for the transfer size. Exceptions are noted in the operation descriptions.

# 8.1.8 Completion Interrupt Handle

Offset: 36; Size: 2 bytes (16 bits)

This field specifies the interrupt table entry to be used to generate a completion interrupt, as described in section 3.7.

This field is reserved if the Request Completion Interrupt flag is 0.

# 8.1.9 Element Count

Offset: 32 for Type Conversion, Reduce, Reduce with Dualcast and Gather Reduce operations

Offset: 40 for Gather Copy, Scatter Copy and Scatter Fill operations

Size: 4 bytes (32 bits)

This field specifies the number of input and/or output data elements of the corresponding data type to read from the source address and/or write to the destination address. The maximum allowed element count is dependent on the WQ that the descriptor was submitted to. The product of element count and the element size specified by the corresponding data type must not exceed the WQ Maximum Transfer Size field for the WQ in the WQ Configuration Table (which is, in turn, limited by the Maximum Supported Transfer Size field in the General Capabilities Register). Element Count must not be 0.

# 8.1.10 Data Types

The data type fields specify the input and output data types for operations that operate on typed data. Data types are specified using the encodings shown in Table 8-5. The Data Types Supported field in DSACAP1 (section 9.2.31) indicates which data types are supported by the implementation.

Some descriptors specify a single data type and some descriptors specify separate input and output data types. For descriptors that specify two data types, the types may be the same or they may be different. If the types are different, they must be either both integer types or both floating point types. If they are floating-point types and they are not the same, then the conversion from IData Type to OData Type must be supported, as indicated by the Floating-Point Conversion Support fields in DSACAPI.

Details about floating-point data types are in Appendix E. The Treat Integer Operands as Signed Values flag in Compute Flags (described in section 8.1.12) is used to differentiate between signed and unsigned integer types. Signed integers are represented in two's complement format.

# 8.1.10.1 Data Type / IData Type

Offset: 56 (bits 3:0); Size: 4 bits

This field specifies the data type of the data to be read for Type Conversion, Reduce, Reduce with Dualcast, Gather Reduce, and Gather Copy operations. It specifies the type of the data to be written for Scatter Copy and Scatter Fill operations.

# 8.1.10.2 OData Type

# Offset: 56 (bits 7:4); Size: 4 bits

This field specifies the data type of the data to be written for Type Conversion, Reduce, Reduce with Dualcast, and Gather Reduce operations.

<table><tr><td>Encoding</td><td>Data Type</td><td>Element Size (bytes)</td></tr><tr><td>0</td><td>UInt8/Int8</td><td>1</td></tr><tr><td>1</td><td>UInt16/Int16</td><td>2</td></tr><tr><td>2</td><td>UInt32/Int32</td><td>4</td></tr><tr><td>3</td><td>UInt64/Int64</td><td>8</td></tr><tr><td>4</td><td>FP8_E5M2</td><td>1</td></tr><tr><td>5</td><td>FP8_E4M3</td><td>1</td></tr><tr><td>6</td><td>FP16</td><td>2</td></tr><tr><td>7</td><td>BF16</td><td>2</td></tr><tr><td>8</td><td>FP32</td><td>4</td></tr><tr><td>9</td><td>FP64</td><td>8</td></tr><tr><td>10-15</td><td>Reserved</td><td>-</td></tr></table>

Table 8-5: Data Types

# 8.1.11 Compute Type

# Offset: 57 (bits 3:0); Size: 4 bits

This field specifies the type of compute operation to be performed on the input data. It is used for Reduce, Reduce with Dualcast, and Gather Reduce operations.

<table><tr><td>Encoding</td><td>Operation (Compute Type)</td><td>Description</td><td>Input Data Types (IData Type)</td><td>Output Data Types (OData Type)</td></tr><tr><td>0</td><td>Reserved</td><td>-</td><td>-</td><td>-</td></tr><tr><td>1</td><td>Add</td><td>Result=Op1 + Op2</td><td>Integer, Floating-Point</td><td>Integer, Floating-Point</td></tr><tr><td>2</td><td>Reserved</td><td>-</td><td>-</td><td>-</td></tr><tr><td>3</td><td>And</td><td>Result=Op1 &amp; Op2</td><td>Integer</td><td>Integer</td></tr><tr><td>4</td><td>Or</td><td>Result=Op1 | Op2</td><td>Integer</td><td>Integer</td></tr><tr><td>5</td><td>Xor</td><td>Result=Op1 ^ Op2</td><td>Integer</td><td>Integer</td></tr><tr><td>6</td><td>Min</td><td>Result=Min(Op1, Op2)</td><td>Integer, Floating-Point</td><td>Integer, Floating-Point</td></tr><tr><td>7</td><td>Max</td><td>Result=Max(Op1, Op2)</td><td>Integer, Floating-Point</td><td>Integer, Floating-Point</td></tr><tr><td>8-15</td><td>Reserved</td><td>-</td><td>-</td><td>-</td></tr></table>


Table 8-6: Compute Operations


# 8.1.12 Compute Flags

# Offset: 57 (bits 7:4) and 58 (bits 7:0); Size: 12 bits

This field specifies flags associated with compute operations. It is used for Type Conversion, Reduce, Reduce with Dualcast, and Gather Reduce operations. Certain flags are only supported with specific data types as indicated in Table 8-7.

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>11:9</td><td>Reserved</td></tr><tr><td>8</td><td>Saturate Integer Result
0: An operation that exceeds the representable range of the output data type results in a wrap-around.
1: An operation that exceeds the representable range of the output data type produces a saturated result. Refer to the description of Numeric Overflow in Table 8-11 for the value returned.
This flag is reserved if the output data type is not an integer type, or if the Saturate Integer Support bit in DSACAP2 is 0 (as described in section 9.2.32).</td></tr><tr><td>7</td><td>Treat Integer Operands as Signed Values
0: Integer operands are treated as unsigned values.
1: Integer operands are treated as signed values represented in two's complement format.
This flag is reserved if neither the input nor output data type is an integer type, or if the Signed Integer Support bit in DSACAP2 is 0 (as described in section 9.2.32).</td></tr><tr><td>6:4</td><td>Rounding Type
000: Round to Nearest Even (RNE)
001: Round Down (RD)
010: Round Up (RU)
011: Round toward Zero (RTZ)
100-111: Reserved
This flag is reserved if neither the input nor output data type is a floating-point type.
The rounding type must be one of the supported types indicated by Rounding Type Support in DSACAP2 (as described in section 9.2.32).</td></tr><tr><td>3</td><td>Treat Denormal as Zero (DAZ)
0: A denormal source operand is not treated as zero. DenormalOperand is reported in the Result field in the completion record.
1: A Denormal source operand is converted to a zero with the sign of the original operand before performing any compute operation with the operand. It is not reported in the Result field of the completion record.
This flag is reserved if the input data type is not a floating-point type, or if the Denormal as Zero Support bit in DSACAP2 is 0 (as described in section 9.2.32).</td></tr><tr><td>2</td><td>Flush to Zero (FTZ)
0: A floating-point underflow condition detected during a compute operation returns a denormal result.
1: A floating-point underflow condition returns a zero value with the sign of the true result.
In either case, Numeric Underflow is reported in the Result field in the completion record.
This flag is reserved if the output data type is not a floating-point type, or if the Flush to Zero Support bit in DSACAP2 is 0 (as described in section 9.2.32).</td></tr><tr><td>1</td><td>Negate Source2Operand
0: Source2Operand is used as is.
1: Negate1 the Source2Operand before performing the compute operation and after any input type conversion, if required.
This field is reserved for Type Conversion and Gather Reduce operations.
This field is reserved if the SourceOperand Negation Support bit in DSACAP2 is 0 (as described in section 9.2.32).</td></tr><tr><td>0</td><td>Negate Source/Source1Operand
0: Source or Source1Operand is used as is.
1: Negate1 the Source or Source1Operand before performing the compute operation and after any input type conversion, if required. For a Gather Reduce operation, the source operands in every input vector are negated.
This field is reserved if the SourceOperand Negation Support bit in DSACAP2 is 0 (as described in section 9.2.32).</td></tr></table>

Table 8-7: Compute Flags

# 8.1.13 Inter-Domain Selector

# Offset: 59; Size: 1 byte (8 bits)

This field is used for Reduce and Reduce with Dualcast operations to specify which of the IDPT handles in the descriptor are used to perform inter-domain operations. It is reserved if the Use Inter-Domain Selector flag in the descriptor is 0. The IDPT Handle 1 and IDPT Handle 2 fields in the descriptor are reserved if they are not used by one of the selectors. Support for inter-domain operations is indicated by the Operations with Inter-Domain Support field in DSACAPO.

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>7:6</td><td>Destination2 Selector
0: No IDPT Handle associated with Destination2. Use descriptor PASID to access Destination2 data.
1: Use IDPT Handle1 to access Destination2 data.
2: Use IDPT Handle2 to access Destination2 data.
3: Reserved
This field is reserved for the Reduce operation.</td></tr><tr><td>4:5</td><td>Destination1 Selector
0: No IDPT Handle associated with Destination1. Use descriptor PASID to access Destination1 data.
1: Use IDPT Handle1 to access Destination1 data.
2: Use IDPT Handle2 to access Destination1 data.
3: Reserved</td></tr><tr><td>3:2</td><td>Source2 Selector
0: No IDPT Handle associated with Source2. Use descriptor PASID to access Source2 data.
1: Use IDPT Handle1 to access Source2 data.
2: Use IDPT Handle2 to access Source2 data.
3: Reserved</td></tr><tr><td>1:0</td><td>Source1 Selector
0: No IDPT Handle associated with Source1. Use descriptor PASID to access Source1 data.
1: Use IDPT Handle1 to access Source1 data.
2: Use IDPT Handle2 to access Source1 data.
3: Reserved</td></tr></table>

Table 8-8: Inter-Domain Selector

# 8.1.14 Scatter Gather List (SGL) Format

# Offset: 59 (bits 7:4); Size: 4 bits

The SGL Format field is used with the Gather Copy, Scatter Copy, Scatter Fill, and Gather Reduce operations. The SGL Format field must specify a supported format, as indicated by the SGL Formats Supported field in DSACAP0, described in section 9.2.30. The encoding for this field is shown in Table 8-9.

Scatter Gather operations allow the source region or destination region to be specified as a list of memory locations instead of a single contiguous address range. The list is known as a Scatter-Gather List (SGL). The number of SGL entries is specified by the SGL Size field in the descriptor. Each SGL entry specifies the address of a buffer, either as a byte-offset or as an index, relative to the Base Address

specified in the descriptor. The Base Address field may be zero. The size of each buffer is determined from other fields of the descriptor.

<table><tr><td>SGL Format</td><td>Description</td></tr><tr><td>0</td><td>Reserved.</td></tr><tr><td>1</td><td>SGL entry specifies 64-bit byte offset.
Buffer Address = Base Address + SGL entry</td></tr><tr><td>2</td><td>SGL entry specifies 32-bit array index.
Buffer Address = Base Address + (SGL entry × Element Count × element size)</td></tr><tr><td>3</td><td>SGL entry specifies 64-bit array index.
Buffer Address = Base Address + (SGL entry × Element Count × element size)</td></tr><tr><td>4-15</td><td>Reserved.</td></tr></table>

Table 8-9: SGL Format

# 8.2 Completion Record

The completion record is a 32-byte structure in memory that the device writes when the operation is complete or encounters an error. A completion record address is in each descriptor. The completion record address must be 32-byte aligned. See section 3.6 for more information.

This section describes fields of the completion record that are common to most operation types. Additional operation-specific fields are described in the detailed operation descriptions in section 8.3. The completion record is always 32 bytes even if not all fields are needed. The completion record contains enough information to continue the operation if it was partially completed due to a page fault. Page faults are indicated by Operation Status codes 0x03, 0x04, 0x06, and 0xff, described in Table 5-6. Software should not depend on the value of unused fields (including fields that are unused for specific operation types).


Generic Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="5">Operation-specific fields</td><td colspan="3">Invalid Flags</td><td>16</td></tr><tr><td colspan="3"></td><td>24</td></tr></table>

# 8.2.1 Status

# Offset: 0; Size: 1 byte (8 bits)

This field reports the completion status of the descriptor. Hardware never writes 0 to this field. Software should initialize this field to 0 so it can detect when the completion record has been written. See section 5.8.1 for a list of the operation status codes and their meanings.

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>7</td><td>R/W (Not used unless Operation Status indicates a translation fault - code 0x03, 0x04, 0x06, 0x1a, or 0x1f)
0: The faulting access was a read.
1: The faulting access was a write.</td></tr><tr><td>6</td><td>Unused.</td></tr><tr><td>5:0</td><td>Operation Status
See section 5.8.1 for the meaning of the value in this field.</td></tr></table>

Table 8-10: Completion Record Status field

# 8.2.2 Result

# Offset: 1; Size: 1 byte (8 bits)

For some operation types, the Result field contains information about the result of the operation. The description of each operation type includes the possible values and meaning of this field. Software should not depend on the value of this field for operation types where no meaning is specified.

For operations types that perform an ALU operation—Type Conversion and Reduce operations—the Result field indicates whether any numeric exceptions were detected during the operation, as shown in Table 8-11. If the operation is successful and the Check Result flag in the descriptor is 1, the Status field of the completion record is set according to Result as shown in Table 8-12.

<table><tr><td>Bit</td><td>Description</td><td>Behavior</td></tr><tr><td>7:4</td><td>Reserved</td><td></td></tr><tr><td>3</td><td>Numeric Underflow (U)
- ALU result rounded to the destination type is less than the smallest finite normal value in that type.</td><td>If the FTZ bit in Compute Operations Flags is 0, return a denominal value.
If the FTZ bit in Compute Operations Flags is 1, return ±0.</td></tr><tr><td>2</td><td>Numeric Overflow (O)
- For FP data types, ALU result rounded to the destination type is greater than the largest finite normal value in that type.
- For integer data types, ALU result converted to the destination type exceeds the representable range of the output data type.</td><td>FP data types:
- For a negative result, if the Rounding Type in Compute Operations Flags is RD or RNE, return -∞1.
- For a positive result, if the Rounding Type in Compute Operations Flags is RU or RNE, return +∞1.
- For other cases, the largest finite number with the appropriate sign is returned.
Integer data types with saturation:
- For signed operands, return the largest positive or negative integer value corresponding to the sign of the true result.
- For unsigned operands, return the largest integer value of that type.
Integer data types without saturation:
- Returns the integer result after wrap-around.</td></tr><tr><td>1</td><td>Denormal Operand (D)
- One or both operands are denormal and the DAZ bit in Compute Operations Flags is 0.</td><td>Result rounded to the destination precision and using the bounded exponent</td></tr><tr><td>0</td><td>Invalid Operation (I)
- Any operation on an operand that is in an unsupported format.
- Any operation on a NaN.
- Addition: operands are opposite-signed infinities.</td><td>Return a NaN result.</td></tr></table>


Table 8-11: Numeric Exception Result Flags


<table><tr><td>Check Result Flag</td><td>Result</td><td>Status</td></tr><tr><td>0</td><td>X</td><td>Success</td></tr><tr><td>1</td><td>0</td><td>Success</td></tr><tr><td>1</td><td>Non-zero</td><td>Success with false predicate</td></tr></table>

Table 8-12: Completion Status for Compute Operations

# 8.2.3 Fault Info

# Offset: 2; Size: 1 byte (8 bits)

If the operation was partially completed due to a page fault and Completion Record Fault Info Support in GENCAP is 1, this field contains additional information about the fault encountered.

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>7:4</td><td>Unused</td></tr><tr><td>3:1</td><td>Operand Identifier
0: Unknown
1: Source, Source1, Descriptor List, Delta Record Address (Apply Delta Record), Drain Readback Address1, or Translation Fetch
2: Source2, CRC Seed Address, or Drain Readback Address2
3: Destination, Destination1, or Delta Record Address (Create Delta Record)
4: Destination2 Address
5: Completion Record Address
6: Scatter-Gather List
7: Reserved</td></tr><tr><td>0</td><td>Fault Address Masked
0: The fault address field contains the address that caused the fault.
1: The fault address is masked or not available.</td></tr></table>

Table 8-13: Completion Record Fault Info

# 8.2.4 Bytes Completed

# Offset: 4; Size: 4 bytes (32 bits)

If the operation was partially completed due to a page fault, this field contains the number of source bytes processed before the fault occurred. All of the source bytes represented by this count were fully processed and the result written to the destination address, as needed according to the operation type. Page faults are indicated by Operation Status codes 0x03, 0x04, 0x06, and 0xff, described in Table 5-6. For other errors, this field is undefined.

For some operation types, this field may also be used when the operation stopped before completion for some reason other than a fault. These uses are described in the section specific to each operation type.

If the operation fully completed, this field is 0.

For operation types where the output size is not readily determinable from this value, the completion record also contains the number of bytes written to the destination address.

# 8.2.5 Fault Address

# Offset: 8; Size: 8 bytes (64 bits)

If the operation was partially completed due to a page fault and Completion Record Fault Info Support in GENCAP is 1, the Fault Info field specifies if the Fault Address is available. If Completion Record Fault Info Support in GENCAP is 0, this field always contains the address that caused the fault. Bits 11:0 may be reported as 0. Page faults are reported as Operation Status codes 0x03, 0x04, 0x06, or 0x1f, described in Table 5-6. For other errors, this field is undefined.

# 8.2.6 Invalid Flags

# Offset: 16; Size: 3 bytes (24 bits)

If the Operation Status is Invalid flags, this field contains a bitmask of the flags that were found to be invalid, to aid in debugging. If a bit in this field is 1, it indicates that the flag at the corresponding bit position in the Flags field of the descriptor was invalid. The implementation may not indicate every invalid flag present in the descriptor, but it indicates at least one flag any time it reports an Invalid flags error code.

If the operation status is anything other than Invalid Flags, this field may be used for operation-specific information, or it may be unused, depending on the operation type. See the description of the completion record for each operation type for more information.

# 8.3 Descriptor Types

# 8.3.1 No-op

The No-op operation, 0x00, performs no DMA operation. It may request a completion record and/or completion interrupt. If it is in a batch, it may specify the Fence flag to ensure that the completion of the No-op descriptor occurs after completion of all previous descriptors in the batch.


No-op Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td rowspan="6" colspan="10">Completion Interrupt Handle</td><td>16</td></tr><tr><td>24</td></tr><tr><td>32</td></tr><tr><td>40</td></tr><tr><td>48</td></tr><tr><td>56</td></tr></table>


No-op Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td rowspan="4" colspan="7">Unused</td><td>Status</td><td>0</td></tr><tr><td></td><td>8</td></tr><tr><td></td><td>16</td></tr><tr><td></td><td>24</td></tr></table>

# 8.3.2 Batch

The Batch operation, 0x01, queues multiple descriptors at once. The Descriptor List Address is the address of a contiguous array of work descriptors to be processed. Each descriptor in the array is 64 bytes. Descriptor List Address must be 64-byte aligned. Descriptor Count is the number of descriptors in the array. The set of descriptors in the array is called the "batch." Descriptor Count must be non-zero. If Batch1 Support in GENCAP is 0, then Descriptor Count must be greater than 1. The maximum number of descriptors allowed in a batch is specified by the WQ Maximum Batch Size field for the WQ in the WQ Configuration Table (which is, in turn, limited by the Maximum Supported Batch Size field in the General Capabilities Register).

The PASID and the Priv flag associated with the Batch descriptor are used for all descriptors in the batch. The PASID and Priv fields in the descriptors in the batch are ignored.

The Descriptors Completed field of the completion record contains the total number of descriptors in the batch that were processed, whether they were successful or not. Descriptors Completed may be less than Descriptor Count if there is a Fence in the batch or if an unrecoverable translation failure occurred while reading the batch.

The Status field of the Batch completion record indicates Success if all of the descriptors in the batch completed successfully; otherwise, it indicates if there was a page fault on the Descriptor List Address or if one or more descriptors in the batch completed with Status not equal to Success.

If Batch Continuation Support in GENCAP is 1, the Result field in the completion record is 1 if any of the operations in the batch completed with Status is not equal to Success. In some cases, Result may be 1 if a descriptor in the batch encountered a page fault on the completion record address, even though the completion record page fault was resolved successfully. Software can examine the completion records for descriptors in the batch to determine whether there were truly any failures.


Batch Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Descriptor List Address</td><td>16</td></tr><tr><td colspan="10"></td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Descriptor Count</td><td>32</td></tr><tr><td colspan="2"></td><td rowspan="3" colspan="8">Reserved</td><td>40</td></tr><tr><td colspan="2"></td><td>48</td></tr><tr><td colspan="2"></td><td>56</td></tr></table>


Batch Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Descriptors Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="8">Unused</td><td>16</td></tr><tr><td>24</td></tr></table>

If software continues execution of a batch after a fault on the Descriptor List Address, it should copy the Result field from the completion record of the partial completion into the Batch Error field of the continuation descriptor

See section 3.8 for details of batch processing.

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>23:17</td><td>Reserved: Must be 0.</td></tr><tr><td>16</td><td>Batch Error
0: This is either a new batch or all operations already completed in this batch completed with Status equal to Success.
1: One or more operations in the batch previously completed with Status not equal to Success. Hardware uses this information in processing of the Fence flag and to determine the Status and Result fields in the Batch completion record.
This flag must be 0 if Batch Continuation Support in GENCAP is 0.</td></tr></table>

Table 8-14:Batch Operation-Specific Flags

# 8.3.3 Drain

The Drain operation,  $0 \times 02$ , waits for completion of certain outstanding descriptors in the WQ that the Drain descriptor is submitted to, as described in section 3.10.

A Drain descriptor may not be included in a batch; it is treated as an unsupported operation type.

Drain must specify Request Completion Record or Request Completion Interrupt. Completion notification is made after the other descriptors have completed.

Table 8-15 lists the operation-specific flags allowed with the Drain operation. The Readback Address 1 Valid and Readback Address 2 Valid flags are reserved if the Drain Descriptor Readback Address Support capability bit is 0.

The flags Address 1 TC Selector, and Address 2 TC Selector are conditionally allowed in the Drain descriptor. Address 1 TC Selector is reserved when Readback Address 1 Valid is 0. Address 2 TC Selector is reserved when Readback Address 2 Valid is 0.


Drain Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Readback Address 1</td><td>16</td></tr><tr><td colspan="10">Readback Address 2</td><td>24</td></tr><tr><td rowspan="4" colspan="2"></td><td rowspan="4" colspan="2">Completion Interrupt Handle</td><td rowspan="4" colspan="6"></td><td>32</td></tr><tr><td>40</td></tr><tr><td>48</td></tr><tr><td>56</td></tr></table>


Drain Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="5"></td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="8">Unused</td><td>16</td></tr><tr><td>24</td></tr></table>

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>23:20</td><td>Reserved: Must be 0.</td></tr><tr><td>19</td><td>Suppress TC-B Implicit Readback
0: Hardware may perform implicit readback on TC-B.
1: Hardware will not perform implicit readback on TC-B. Note that this flag does not affect readbacks to the explicit Readback Addresses.</td></tr><tr><td>18</td><td>Suppress TC-A Implicit Readback
0: Hardware may perform implicit readback on TC-A.
1: Hardware will not perform implicit readback on TC-A. Note that this flag does not affect readbacks to the explicit Readback Addresses.</td></tr><tr><td>17</td><td>Readback Address 2 Valid
0: Readback Address 2 field is reserved.
1: Readback Address 2 field is valid and hardware will perform a readback to this address on the TC specified by the Address 2 TC Selector flag. Note that the cache control flags are reserved for Drain descriptors.</td></tr><tr><td>16</td><td>Readback Address 1 Valid
0: Readback Address 1 field is reserved.
1: Readback Address 1 field is valid and hardware will perform a readback to this address on the TC specified by the Address 1 TC Selector flag. Note that the cache control flags are reserved for Drain descriptors.</td></tr></table>

Table 8-15: Drain Operation-Specific Flags

# 8.3.4 Memory Move

The Memory Move operation, 0x03, copies memory from the Source Address to the Destination Address. The number of bytes copied is given by Transfer Size. There are no alignment requirements for the memory addresses or the transfer size.

If the source and destination regions overlap, the behavior depends on the value of the Overlapping Copy Support field in GENCAP. If Overlapping Copy Support is 1, the memory copy is done as if the entire source buffer is copied to temporary space and then copied to the destination buffer. (This may be implemented by reversing the direction of the copy when the beginning of the destination buffer overlaps the end of the source buffer.) If Overlapping Copy Support is 0, it is an error.

If the operation is partially completed due to a page fault, the Result field of the completion record contains the direction of the copy. It is 0 if the copy was performed starting at the beginning of the source and destination buffers; it is 1 if the direction of the copy was reversed. If Overlapping Copy Support is 0, Result is always 0.

To resume the operation after a partial completion, if Result is 0, the Source and Destination Address fields in the continuation descriptor should be increased by Bytes Completed, and the Transfer Size should be decreased by Bytes Completed. If Result is 1, the Transfer Size should be decreased by Bytes Completed, but the Source and Destination Address fields should be the same as in the original descriptor. Note that if a subsequent partial completion occurs, the Result field is not necessarily the same as it was for the first partial completion.


Memory Move Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td colspan="2">Reserved</td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td rowspan="3" colspan="10">Reserved</td><td>40</td></tr><tr><td>48</td></tr><tr><td>56</td></tr></table>


Memory Move Descriptor Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="8">Reserved</td><td>16</td></tr><tr><td>24</td></tr></table>

# 8.3.5 Fill

The Memory Fill operation, 0x04, fills memory at the Destination Address with the value in the pattern field. The pattern size is specified by the Pattern Size flag. When the pattern size is 8 bytes, the pattern is specified in the Pattern Lower field. When the pattern size is 16 bytes, the first 8 bytes are in Pattern Lower and the next 8 bytes are in Pattern Upper. (To use a smaller pattern, software must replicate the pattern in the descriptor.) The number of bytes written is given by Transfer Size. The transfer size does not need to be a multiple of the pattern size. There are no alignment requirements for the destination address or the transfer size. If the operation is partially completed due to a page fault, the Bytes Completed field of the completion record contains the number of bytes written to the destination before the fault occurred.


Fill Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Pattern Lower</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="10">Pattern Upper</td><td>40</td></tr><tr><td rowspan="2" colspan="10">Reserved</td><td>48</td></tr><tr><td>56</td></tr></table>


Fill Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="8">Unused</td><td>16</td></tr><tr><td>24</td></tr></table>

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>23:19</td><td>Reserved: Must be 0.</td></tr><tr><td>18</td><td>Pattern Size
0: Pattern size is 8B and specified in the Pattern Lower field. The Pattern Upper field is reserved.
1: Pattern size is 16B and specified by the Pattern Lower and Pattern Upper fields. This field must be 0 if Fillló Support in GENCAP is 0.</td></tr><tr><td>17:16</td><td>Reserved: Must be 0.</td></tr></table>

Table 8-16 : Fill Operation-Specific Flags

# 8.3.6 Compare

The Compare operation, 0x05, compares memory at Source1 Address with memory at Source2 Address. The number of bytes compared is given by Transfer Size. There are no alignment requirements for the memory addresses or the transfer size. The Completion Record Address Valid and Request Completion Record flags must be 1 and the Completion Record Address must be valid. The result of the comparison is written to the Result field of the completion record: a value of 0 indicates that the two memory regions match, and a value of 1 indicates that they do not match. If Result is 1, the Bytes Completed field of the completion record indicates the byte offset of the first difference. If the operation is partially completed due to a page fault, Result is 0. (If a difference had been detected, the difference would be reported instead of the page fault.)


Compare Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source1 Address</td><td>16</td></tr><tr><td colspan="10">Source2 Address</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="2"></td><td rowspan="3" colspan="6">Reserved</td><td colspan="2">Expected Result</td><td>40</td></tr><tr><td colspan="2"></td><td colspan="2"></td><td>48</td></tr><tr><td colspan="2"></td><td colspan="2"></td><td>56</td></tr></table>


Compare Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="8">Unused</td><td>16</td></tr><tr><td>24</td></tr></table>

If the operation is successful and the Check Result flag is 1, the Status field of the completion record is set according to Result and Expected Result, as shown in Table 8-17. This allows a subsequent descriptor in the same batch with the Fence flag to continue or stop execution of the batch based on the result of the comparison. Bits 7:1 of Expected Result are ignored.

<table><tr><td>Check Result flag</td><td>Expected Result bit 0</td><td>Result</td><td>Status</td></tr><tr><td>0</td><td>X</td><td>X</td><td>Success</td></tr><tr><td>1</td><td>0</td><td>0</td><td>Success</td></tr><tr><td>1</td><td>0</td><td>1</td><td>Success with false predicate</td></tr><tr><td>1</td><td>1</td><td>0</td><td>Success with false predicate</td></tr><tr><td>1</td><td>1</td><td>1</td><td>Success</td></tr></table>

Table 8-17: Completion Status for Compare Descriptor

# 8.3.7 Compare Pattern

The Compare Pattern operation, 0x06, compares memory at Source Address with the value in the pattern field. The pattern size is always 8 bytes. (To use a smaller pattern, software must replicate the pattern in the descriptor.) The number of bytes compared is given by Transfer Size. The transfer size does not need to be a multiple of the pattern size. The Completion Record Address Valid and Request Completion Record flags must be 1 and the Completion Record Address must be valid. The result of the comparison is written to the Result field of the completion record; a value of 0 indicates that the memory region matches the pattern, and a value of 1 indicates that it does not match. If Result is 1, the Bytes Completed field of the completion record indicates the location of the first difference. (It may not be the exact byte location, but it is guaranteed to be no greater than the first difference.) If the operation is partially completed due to a page fault, Result is 0. (If a difference had been detected, the difference would be reported instead of the page fault.)

The completion record format for Compare Pattern and the behavior of Check Result and Expected Result are identical to Compare.


Compare Pattern Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Pattern</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="2"></td><td rowspan="3" colspan="6">Reserved</td><td>Expected Result</td><td>40</td><td></td></tr><tr><td colspan="2"></td><td></td><td>48</td><td></td></tr><tr><td colspan="2"></td><td></td><td>56</td><td></td></tr></table>


Compare Pattern Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="8">Unused</td><td>16</td></tr><tr><td>24</td></tr></table>

# 8.3.8 Create Delta Record

The Create Delta Record operation, 0x07, compares memory at Source1 Address with memory at Source2 Address and generates a delta record that contains the information needed to update source1 to match source2. The number of bytes compared is given by Transfer Size. The transfer size is limited by the maximum offset that can be stored in the delta record, as described below, in addition to the usual WQ-specific limit on transfer size. Source1 Address, Source2 Address, and Transfer Size must be aligned to a multiple of 8. The Completion Record Address Valid and Request Completion Record flags must be 1 and the Completion Record Address must be valid.

The maximum size of the delta record is given by Maximum Delta Record Size. The maximum delta record size should be a multiple of the delta size (10 bytes), must not be less than the maximum number of deltas that can be generated from a single cache line (80 bytes), and must be no greater than the value allowed by the WQ Maximum Transfer Size in the WQ Configuration Table of the WQ that this descriptor was submitted to. If the maximum-size delta record overlaps either of the source buffers, it is an error. The actual size of the delta record that is generated depends on the number of differences detected between source1 and source2; this size is written to the Delta Record Size field of the completion record. If the space needed in the delta record exceeds the maximum delta record size specified in the descriptor, the operation completes with a partial delta record.

The result of the comparison is written to the Result field of the completion record. If the two regions match exactly, then Result is 0, Delta Record Size is 0, and Bytes Completed is 0. If the two regions don't match, and a complete set of deltas was written to the delta record, then Result is 1, Delta Record Size contains the total size of all the differences found, and Bytes Completed is 0. If the two regions don't match, and the space needed to record all the deltas exceeded the maximum delta record size, then Result is 2, Delta Record Size contains the size of the set of deltas written to the delta record (typically equal or nearly equal to the Maximum Delta Record Size specified in the descriptor), and Bytes Completed contains the number of bytes compared before space in the delta record was exceeded.


Create Delta Record Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source1 Address</td><td>16</td></tr><tr><td colspan="10">Source2 Address</td><td>24</td></tr><tr><td colspan="2">Reserved</td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="10">Delta Record Address</td><td>40</td></tr><tr><td colspan="4"></td><td colspan="6">Maximum Delta Record Size</td><td>48</td></tr><tr><td colspan="9">Reserved</td><td>Expected Result Mask</td><td>56</td></tr></table>


Create Delta Record Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="4"></td><td colspan="4">Delta Record Size</td><td>16</td></tr><tr><td colspan="8">Unused</td><td>24</td></tr></table>

If the operation is partially completed due to a page fault, Result is set to 0 if no deltas were written prior to the page fault, and Result is set to 1 if any deltas were written prior to the page fault. This behavior is the same whether the page fault is on one of the source buffers or on the delta record buffer. Bytes Completed contains the number of bytes compared before the page fault occurred, and Delta Record Size contains the space used in the delta record before the page fault occurred. If the operation fails due to any other error, these fields are undefined. To ensure that software can resume the operation without losing any deltas, if the fault occurred on the delta record, Bytes Completed does not include the bytes where the difference was found that was not written to the delta record.

The format of the delta record is shown below. The delta record contains an array of deltas. Each delta contains a 2-byte offset and an 8-byte block of data from Source2 that is different from the corresponding 8 bytes in Source1. The 2-byte offset field is stored in memory with the low byte at the lower address (little-endian). The total size of the delta record is a multiple of 10. Since the offset is a 16-bit field representing a multiple of 8 bytes, the maximum offset that can be expressed is 0x7FFF8, so the maximum Transfer Size is 0x80000 bytes (512 KB).

<table><tr><td>Byte
0</td><td>1</td><td>2</td><td>3</td><td>4</td><td>5</td><td>6</td><td>7</td><td>8</td><td>Byte
9</td></tr><tr><td colspan="2">Offset</td><td colspan="8">Data</td></tr><tr><td>LSB</td><td>MSB</td><td>Byte 0</td><td>Byte 1</td><td>Byte 2</td><td>Byte 3</td><td>Byte 4</td><td>Byte 5</td><td>Byte 6</td><td>Byte 7</td></tr></table>

If the operation is successful and the Check Result flag is 1, the Status field of the completion record is set according to Result and Expected Result Mask. This allows a subsequent descriptor in the same batch with the Fence flag to continue or stop execution of the batch based on the result of the delta record creation. Status is set as follows: If the value of Result is X and bit X of the Expected Results Mask is 1, Status is set to Success. If bit X is 0, Status is set to Success with false predicate. Since the value of Result is 0, 1, or 2, bits 7:3 of Expected Result Mask are ignored. Note that if bits 2:0 of Expected Result Mask are 0, Status will always be set to Success with false predicate, and if bits 2:0 of Expected Result Mask are all 1, Status will always be set to Success.

If the operation is successful and the Check Result flag is 0, the Expected Result Mask is ignored and Status is set to Success.

# 8.3.9 Apply Delta Record

The Apply Delta Record operation,  $0 \times 08$ , applies a delta record to the contents of memory at Destination Address. Delta Record Address is the address of a delta record that was created by a Create Delta Record operation that completed with Result equal to 1. Delta Record Size is the size of the delta record, as reported in the completion record of the Create Delta Record operation. Destination Address is the address of a buffer that contains the same contents as the memory at the Source1 Address when the delta record was created. Transfer Size is the same as the Transfer Size used when the delta record was created. After the Apply Delta Record operation completes, the memory at Destination Address will match the contents that were in memory at the Source2 Address when the delta record was created. Destination Address and Transfer Size must be aligned to a multiple of 8. If the delta record overlaps the destination buffer, it is an error.

If a page fault is encountered during the Apply Delta Record operation, the Bytes Completed field of the completion record contains the number of bytes of the delta record that were successfully applied to the destination. If software chooses to submit another descriptor to resume the operation, the continuation descriptor should contain the same Destination Address as the original. The Delta Record Address should be increased by Bytes Completed (so it points to the first unapplied delta), and the Delta Record Size should be reduced by Bytes Completed.

If the offset fields in the delta record are not in ascending order, or if any offset field is greater than or equal to Transfer Size, an error is reported and the Bytes Completed field of the completion record contains the number of bytes of the delta record that were successfully applied to the destination prior to the error.

See section 8.3.8 for a description of the format of the delta record.


Apply Delta Record Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Delta Record Address</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td colspan="2">Reserved</td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="4"></td><td colspan="6">Delta Record Size</td><td>40</td></tr><tr><td colspan="4"></td><td colspan="6"></td><td>48</td></tr><tr><td colspan="4"></td><td colspan="6">Reserved</td><td>56</td></tr></table>


Apply Delta Record Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td></td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="8">Unused</td><td>16</td></tr><tr><td>24</td></tr></table>


Figure 8-1 shows the usage of the Create Delta Record and Apply Delta Record operations. First, the Create Delta Record operation is performed. It reads the two source buffers and writes the delta record, recording the actual delta record size in its completion record. The Apply Delta Record operation takes the content of the delta record that was written by the Create Delta Record operation, along with its size and a copy of the Source1 data, and updates the destination buffer to be a duplicate of the original Source2 buffer.


![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/a6d22f37ee8c249c0f144c940389f075f6ab4fb8a63374b0f7cc7e3de7981aa0.jpg)



Figure 8-1: Delta Record Usage


# 8.3.10 Memory Copy with Dualcast

The Memory Copy with Dualcast operation, 0x09, copies memory from the Source Address to both Destination1 Address and Destination2 Address. The number of bytes copied is given by Transfer Size. There are no alignment requirements for the source address or the transfer size. Bits 11:0 of the two destination addresses must be the same.

If the source region overlaps with either of the destination regions or if the two destination regions overlap, it is an error. If the operation is partially completed due to a page fault, the copy operation stops after having written the same number of bytes to both destination regions.


Memory Copy with Dualcast Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Destination1 Address</td><td>24</td></tr><tr><td colspan="2">Reserved</td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="10">Destination2 Address</td><td>40</td></tr><tr><td rowspan="2" colspan="10">Reserved</td><td>48</td></tr><tr><td>56</td></tr></table>


Memory Copy with Dualcast Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td></td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="8">Unused</td><td>16</td></tr><tr><td>24</td></tr></table>

# 8.3.11 Translation Fetch

The Translation Fetch operation, 0x0A, fetches address translations for the address range specified in the descriptor by issuing address translation (ATS) requests to the IOMMU. There is no data movement associated with this operation. The Region Size field specifies the size of the address range over which the translation requests may be issued. If the Use Stride flag is 1, the Region Stride field specifies the number of bytes to skip to compute the address for each subsequent ATS request. The Region Stride must be greater than or equal to 4096 and must be a power of 2. The descriptor execution terminates when the ATS request address is greater than or equal to the starting address plus region size. If Use Stride is 0, the device uses an implementation specific stride value. There are no alignment requirements for the address or the region size. However, the translation requests are always 4KB aligned and may be aligned to a multiple of the Region Stride. Alignment may result in a translation request outside the specified region.

The Region Size must be non-zero, and the sum of Address and Region Size in the descriptor must be less than or equal to  $2^{64}$ . In case a page fault is encountered, the Block on Fault flag controls whether the operation is partially completed, or a page request is issued to the IOMMU. In case of partial completion due to a page fault, the fault address is reported in the completion record. The Bytes Completed field is undefined and should not be relied upon by software.

The operation may result in one or more of the address translations performed by the IOMMU, or translation structure entries used for address translation, or both, to be cached in an Address Translation Cache in the device or in the IOMMU.


Translation Fetch Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Address</td><td>16</td></tr><tr><td colspan="10"></td><td>24</td></tr><tr><td colspan="2"></td><td rowspan="4" colspan="2">Completion Interrupt Handle</td><td colspan="6">Region Size</td><td>32</td></tr><tr><td colspan="2"></td><td colspan="6">Reserved</td><td>40</td></tr><tr><td colspan="2"></td><td colspan="6">Region Stride</td><td>48</td></tr><tr><td colspan="2"></td><td colspan="6"></td><td>56</td></tr></table>


Translation Fetch Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="5">Unused</td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="8">Unused</td><td>16</td></tr><tr><td>24</td></tr></table>

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>23:17</td><td>Reserved: Must be 0.</td></tr><tr><td>16</td><td>Use Stride
0: The Region Stride field is reserved. Hardware uses an implementation specific value.
1: The Region Stride field specifies the number of bytes to skip to compute the address for each subsequent ATS request.
This field is reserved if Translation Fetch Stride Support in GENCAP is 0.</td></tr></table>

Table 8-18 : Translation Fetch Operation-Specific Flags

# 8.3.12 CRC Generation

The CRC Generation operation, 0x10, computes the CRC on memory at the Source Address. See Appendix A for details of CRC Generation. The number of bytes used for the CRC computation is given by Transfer Size. There are no alignment requirements for the memory addresses or the transfer size. The Completion Record Address Valid and Request Completion Record flags must be 1 and the Completion Record Address must be valid. The computed CRC value is written to the completion record.

The CRC Generation operation-specific flags are shown in Table 8-19. The size of the CRC value computed, and the CRC seed used depends on the CRC Size flag. If CRC Size is 0, then a 32-bit CRC seed is used, and a 32-bit CRC value is computed. If CRC Size is 1, then a 64-bit CRC seed is used, and a 64-bit CRC value is computed.

If the Read CRC Seed flag is 1, the CRC seed is read from memory at the CRC Seed Address. The address must be naturally aligned to the size of the CRC value. If the Read CRC Seed flag is 0, the CRC Seed field in the descriptor is used for the seed. Unless this is a continuation of a partial CRC computation, the seed should be 0. If CRC Size is 0, bits 63:32 of CRC Seed are reserved.

If the operation is partially completed due to a page fault, the partial CRC result is written to the completion record along with the page fault information. If software corrects the fault and resumes the operation, it must use the partial CRC result as the seed of the continuation descriptor, either by copying it into the CRC Seed field or by setting the CRC Seed Address to the location of the partial CRC result and setting the Read CRC Seed flag to 1. If the operation fails due to any other error, or if Bytes Completed is 0, the CRC Value in the completion record is undefined and software should reuse the CRC Seed or CRC Seed Address from the descriptor.

If the Read CRC Seed flag is 0, the CRC Seed Address field is reserved. If the Read CRC Seed flag is 1, the CRC Seed field is reserved.


CRC Generation Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="2">Reserved</td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>24</td></tr><tr><td colspan="10">CRC Seed</td><td>40</td></tr><tr><td colspan="10">CRC Seed Address</td><td>48</td></tr><tr><td colspan="10">Reserved</td><td>56</td></tr></table>


CRC Generation Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="8">CRC Value</td><td>16</td></tr><tr><td colspan="8">Unused</td><td>24</td></tr><tr><td colspan="8">Bits</td><td>Description</td></tr><tr><td>23:20</td><td colspan="8">Reserved: Must be 0.</td></tr><tr><td>19</td><td colspan="8">CRC Size
0: 32-bit CRC value is computed. Upper 4 bytes of CRC Seed field in the descriptor and the CRC Value field in the completion record are reserved.
1: 64-bit CRC value is computed.
This flag must be 0 if CRC64 Support in GENCAP is 0.
See Appendix A for details of CRC computation.</td></tr><tr><td>18</td><td colspan="8">Bypass Data Reflection
0: Normal CRC operation: bit 0 of each data byte is the MSB in the CRC computation.
1: Bit 7 of each data byte is the MSB in the CRC computation.
See Appendix A for details of CRC computation.</td></tr><tr><td>17</td><td colspan="8">Bypass CRC Inversion and Reflection
0: Normal CRC operation: CRC seed and result are inverted and use standard CRC bit order.
1: Bypass inversion and use reverse bit order for CRC seed and result.
See Appendix A for details of CRC computation.</td></tr><tr><td>16</td><td colspan="8">Read CRC Seed
0: Use the CRC Seed field in the descriptor.
1: Read the CRC seed from memory at the CRC Seed Address.</td></tr></table>

Table 8-19: CRC Generation Operation-Specific Flags

# 8.3.13 Copy with CRC Generation

The Copy with CRC Generation operation, 0x11, copies memory from the Source Address to the Destination Address and computes the CRC on the data copied. See Appendix A for details of CRC Generation. The number of bytes copied is given by Transfer Size. There are no alignment requirements for the memory addresses or the transfer size. If the source and destination regions overlap, it is an error. The Completion Record Address Valid and Request Completion Record flags must be 1 and the Completion Record Address must be valid. The computed CRC value is written to the completion record.

See the description of the CRC Generation operation and Table 8-19 in section 8.3.12 for a description of the CRC operation-specific flags, the CRC Seed field, and the CRC Seed Address field.

The completion record format for Copy with CRC Generation is identical to the format for CRC Generation.


Copy with CRC Generation Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="10">CRC Seed</td><td>40</td></tr><tr><td colspan="10">CRC Seed Address</td><td>48</td></tr><tr><td colspan="10">Reserved</td><td>56</td></tr></table>


Copy with CRC Generation Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="8">CRC Value</td><td>16</td></tr><tr><td colspan="8">Unused</td><td>24</td></tr></table>

# 8.3.14 DIF Check

The DIF Check operation, 0x12, computes the Data Integrity Field (DIF) on the source data and compares the computed DIF to the DIF contained in the source data.

The number of source bytes read is given by Transfer Size. DIF computation is performed on each block of source data that is 512, 520, 4096, or 4104 bytes. The transfer size should be a multiple of the source block size plus 8 bytes for each source block. There is no alignment requirement for the source address.

If the operation completes successfully, the final Reference Tag and Application Tag are written to the completion record along with a Success completion status. If the operation is partially completed due to a page fault, updated values of Reference Tag and Application Tag are written to the completion record along with the page fault information. If software corrects the fault and resumes the operation, it may copy these fields into the continuation descriptor. If the operation fails due to any other error, these fields are undefined.

If an error is detected in the DIF in the source data, the operation stops. The Status field in the completion record is set to DIF Error, the DIF Status field is set to indicate the type of error, and the Bytes Completed field is set to the number of source bytes successfully processed. Bytes Completed does not include the block in which the error was detected. The Completion Record Address Valid and Request Completion Record flags must be 1 and the Completion Record Address must be valid.

See section 8.3.17, DIF Update, for a description of DIF Flags, Source DIF Flags, and the fields in the completion record. See Appendix B for details of DIF checking.


DIF Check Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Reserved</td><td>24</td></tr><tr><td></td><td colspan="3">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td></td><td colspan="4"></td><td colspan="3">DIF Flags</td><td colspan="2">Source DIF Flags</td><td>40</td></tr><tr><td colspan="2">Application Tag Seed</td><td colspan="2">Application Tag Mask</td><td colspan="6">Reference Tag Seed</td><td>48</td></tr><tr><td colspan="10"></td><td>56</td></tr></table>


DIF Check Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>DIF Status</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="2">Application Tag</td><td colspan="2">Application Tag Mask</td><td colspan="4">Reference Tag</td><td>16</td></tr><tr><td colspan="8">Unused</td><td>24</td></tr></table>

# 8.3.15 DIF Insert

The DIF Insert operation, 0x13, copies memory from the Source Address to the Destination Address, while computing the Data Integrity Field (DIF) on the source data and inserting the DIF into the output data.

The number of source bytes copied is given by Transfer Size. DIF computation is performed on each block of source data that is 512, 520, 4096, or 4104 bytes. The transfer size should be a multiple of the source block size. The number of bytes written to the destination is the transfer size plus 8 bytes for each source block. There is no alignment requirement for the memory addresses. If the source and destination regions overlap, it is an error.

If the operation completes successfully, the final Reference Tag and Application Tag are written to the completion record along with a Success completion status. If the operation is partially completed due to a page fault, updated values of Reference Tag and Application Tag are written to the completion record along with the page fault information. If software corrects the fault and resumes the operation, it may copy these fields into the continuation descriptor. If the operation fails due to any other error, these fields are undefined.

See section 8.3.17, DIF Update, for a description of DIF Flags, Destination DIF Flags, and the fields in the completion record. See Appendix B for details of DIF computation.


DIF Insert Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td rowspan="3" colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td rowspan="2" colspan="4">Reserved</td><td rowspan="2" colspan="2">DIF Flags</td><td rowspan="2">Dest DIF Flags</td><td></td><td>40</td></tr><tr><td></td><td>48</td></tr><tr><td colspan="2">Application Tag Seed</td><td colspan="2">Application Tag Mask</td><td colspan="6">Reference Tag Seed</td><td>56</td></tr></table>


DIF Insert Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td></td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="8">Unused</td><td>16</td></tr><tr><td colspan="2">Application Tag</td><td colspan="2">Application Tag Mask</td><td colspan="4">Reference Tag</td><td>24</td></tr></table>

# 8.3.16 DIF Strip

The DIF Strip operation, 0x14, copies memory from the Source Address to the Destination Address, removing the Data Integrity Field (DIF). It optionally computes the DIF on the source data and compares the computed DIF to the DIF contained in the source data.

The number of source bytes read is given by Transfer Size. DIF computation is performed on each block of source data that is 512, 520, 4096, or 4104 bytes. The transfer size should be a multiple of the source block size plus 8 bytes for each source block. The number of bytes written to the destination is the transfer size minus 8 bytes for each source block. There is no alignment requirement for the memory addresses. If the source and destination regions overlap, it is an error.

If the operation completes successfully, the final Reference Tag and Application Tag are written to the completion record along with a Success completion status. If the operation is partially completed due to a page fault, updated values of Reference Tag and Application Tag are written to the completion record along with the page fault information. If software corrects the fault and resumes the operation, it may copy these fields into the continuation descriptor. If the operation fails due to any other error, these fields are undefined.

If an error is detected in the DIF in the source data, the operation stops. The Status field in the completion record is set to DIF Error, the DIF Status field is set to indicate the type of error, and the Bytes Completed field is set to the number of source bytes successfully processed. Bytes Completed does not include the block in which the error was detected.

See section 8.3.17, DIF Update, for a description of DIF Flags, Source DIF Flags, and the fields in the completion record. See Appendix B for details of DIF checking.


DIF Strip Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td colspan="2">Reserved</td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="5">Reserved</td><td colspan="2">DIF Flags</td><td>Reserved</td><td colspan="2">Source DIF Flags</td><td>40</td></tr><tr><td colspan="2">Application Tag Seed</td><td colspan="2">Application Tag Mask</td><td colspan="6">Reference Tag Seed</td><td>48</td></tr><tr><td colspan="10">Reserved</td><td>56</td></tr></table>


DIF Strip Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>DIF Status</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="2">Application Tag</td><td colspan="2">Application Tag Mask</td><td colspan="4">Reference Tag</td><td>16</td></tr><tr><td colspan="8">Unused</td><td>24</td></tr></table>

# 8.3.17 DIF Update

The DIF Update operation, 0x15, copies memory from the Source Address to the Destination Address. It optionally computes the Data Integrity Field (DIF) on the source data and compares the computed DIF to the DIF contained in the data. It simultaneously computes the DIF on the source data using Destination DIF Flags in the descriptor and inserts the computed DIF into the output data.

The number of source bytes read is given by Transfer Size. DIF computation is performed on each block of source data that is 512, 520, 4096, or 4104 bytes. The transfer size should be a multiple of the source block size plus 8 bytes for each source block. The number of bytes written to the destination is the same as the transfer size. There is no alignment requirement for the memory addresses. If the source and destination regions overlap, it is an error.

If the operation completes successfully, the final source and destination Reference Tags and Application Tags are written to the completion record along with a Success completion status. If the operation is partially completed due to a page fault, updated values of the source and destination Reference Tags and Application Tags are written to the completion record along with the page fault information. If software corrects the fault and resumes the operation, it may copy these fields into the continuation descriptor. If the operation fails due to any other error, these fields are undefined.

If an error is detected in the DIF in the source data, the operation stops. The Status field in the completion record is set to DIF Error, the DIF Status field is set to indicate the type of error, and the Bytes Completed field is set to the number of source bytes successfully processed (including generated DIF bytes). Bytes Completed does not include the block in which the error was detected.

See Appendix B for details of DIF computation and checking.


DIF Update Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td colspan="2">Reserved</td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="6">Reserved</td><td colspan="2">DIF Flags</td><td>Dest DIF Flags</td><td>Source DIF Flags</td><td>40</td></tr><tr><td colspan="2">Source Application Tag Seed</td><td colspan="2">Source Application Tag Mask</td><td colspan="6">Source Reference Tag Seed</td><td>48</td></tr><tr><td colspan="2">Destination Application Tag Seed</td><td colspan="2">Destination Application Tag Mask</td><td colspan="6">Destination Reference Tag Seed</td><td>56</td></tr></table>


DIF Update Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>DIF Status</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="2">Source Application Tag</td><td colspan="2">Source Application Tag Mask</td><td colspan="4">Source Reference Tag</td><td>16</td></tr><tr><td colspan="2">Destination Application Tag</td><td colspan="2">Destination Application Tag Mask</td><td colspan="4">Destination Reference Tag</td><td>24</td></tr></table>

# 8.3.17.1 DIF Flags

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>7:4</td><td>Reserved.</td></tr><tr><td>3</td><td>Invert CRC Result
0: Do not invert CRC result.
1: Invert CRC result. (That is, invert each bit of the final CRC value.)</td></tr><tr><td>2</td><td>Invert CRC Seed
0: The initial seed is 0.
1: The initial seed is 0xFFFF.</td></tr><tr><td>1:0</td><td>DIF Block Size
00b: 512 bytes
01b: 520 bytes
10b: 4096 bytes
11b: 4104 bytes</td></tr></table>

# 8.3.17.2 Source DIF Flags

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>7</td><td>Source Reference Tag TypeThis field denotes the type of operation to perform on the source DIF Reference Tag.0:Incrementing1:Fixed</td></tr><tr><td>6</td><td>Reference Tag Check Disable0:Enable Reference Tag field checking.1:Disable Reference Tag field checking.</td></tr><tr><td>5</td><td>Guard Check Disable0:Enable Guard field checking.1:Disable Guard field checking.</td></tr><tr><td>4</td><td>Source Application Tag TypeThis field denotes the type of operation to perform on the source DIF Application Tag.0:Fixed1:IncrementingNote that the meaning of the Application Tag Type is reversed compared to the Reference Tag Type. The default typically used in storage systems is for the Application Tag to be fixed and the Reference Tag to be incrementing.</td></tr><tr><td>3</td><td>Application and Reference Tag F Detect0: Disable F Detect for Application Tag and Reference Tag fields.1: Enable F Detect for Application Tag and Reference Tag fields. When all bits of both the Application Tag and Reference Tag fields are equal to 1, the Application Tag and Reference Tag checks are not done and the Guard field is ignored.</td></tr><tr><td>2</td><td>Application Tag F Detect0: Disable F Detect for the Application Tag field.1: Enable F Detect for the Application Tag field. When all bits of the Application Tag field of the source Data Integrity Field are equal to 1, the Application Tag check is not done and the Guard field and Reference Tag field are ignored.</td></tr><tr><td>1</td><td>All F Detect
0: Disable All F Detect.
1: Enable All F Detect. When all bits of the Application Tag, Reference Tag, and Guard fields are equal to 1, no checks are performed on these fields. (The All F Detect Status is reported, if enabled.)</td></tr><tr><td>0</td><td>Enable All F Detect Error
0: Disable All F Detect Error.
1: Enable All F Detect Error. When all bits of the Application Tag, Reference Tag, and Guard fields are equal to 1, All F Detect Error is reported in the DIF Status field of the Completion Record.
If All F Detect flag is 0, this flag is ignored.</td></tr></table>

# 8.3.17.3 Destination DIF Flags

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>7</td><td>Destination Reference Tag TypeThis field denotes the type of operation to perform on the destination DIF Reference Tag.0:Incrementing1:Fixed</td></tr><tr><td>6</td><td>Reference Tag Pass-through0:The Reference Tag field written to the destination is determined based on the Destination Reference Tag Seed and Destination Reference Tag Type fields of the descriptor.1:The Reference Tag field from the source is copied to the destination. The Destination Reference Tag Seed and Destination Reference Tag Type fields of the descriptor are ignored.This field is ignored for the DIF Insert and DIX Generate operations.</td></tr><tr><td>5</td><td>Guard Field Pass-through0:The Guard field written to the destination is computed from the source data.1:The Guard field from the source is copied to the destination.This field is ignored for the DIF Insert and DIX Generate operations.</td></tr><tr><td>4</td><td>Destination Application Tag TypeThis field denotes the type of operation to perform on the destination DIF Application Tag.0:Fixed1:IncrementingNote that the meaning of the Application Tag Type is reversed compared to the Reference Tag Type. The default typically used in storage systems is for the Application Tag to be fixed and the Reference Tag to be incrementing.</td></tr><tr><td>3</td><td>Application Tag Pass-through0:The Application Tag field written to the destination is determined based on the Destination Application Tag Seed, Destination Application Tag Mask, and Destination Application Tag Type fields of the descriptor.1:The Application Tag field from the source is copied to the destination. The Destination Application Tag Seed, Destination Application Tag Mask, and Destination Application Tag Type fields of the descriptor are ignored.This field is ignored for the DIF Insert and DIX Generate operations.</td></tr><tr><td>2:0</td><td>Reserved.</td></tr></table>

# 8.3.17.4 DIF Status

# Completion Record Offset: 1; Size: 1 byte

This field reports the status of a DIF operation. This field is defined only for DIF Check, DIF Strip, and DIF Update operations and only if the Status field of the Completion Record is DIF Error. The values 0x01, 0x02, and 0x04 may be combined when more than one error is detected for a single block.

<table><tr><td>0x01</td><td>Guard mismatch. This value is reported under the following condition:
- Guard Check Disable is 0;
- F Detect condition is not detected; and
- The guard value computed from the source data does not match the Guard field in the source Data Integrity Field.</td></tr><tr><td>0x02</td><td>Application Tag mismatch. This value is reported under the following condition:
- Source Application Tag Mask is not equal to 0xFFFF;
- F Detect condition is not detected; and
- The computed Application Tag value does not match the Application Tag field in the source Data Integrity Field.</td></tr><tr><td>0x04</td><td>Reference Tag mismatch. This value is reported under the following condition:
- Reference Tag Check Disable is 0.
- F Detect condition is not detected; and
- The computed Application Tag value does not match the Application Tag field in the source Data Integrity Field.</td></tr><tr><td>0x08</td><td>All F Detect Error. This value is reported under the following condition:
- All F Detect is 1;
- Enable All F Detect Error is 1;
- All bits of the Application Tag, Reference Tag, and Guard fields of the source Data Integrity Field are equal to 1.</td></tr></table>

F Detect condition is detected when one of the following is true:

<table><tr><td>All F Detect = 1</td><td>All bits of the Application Tag, Reference Tag, and Guard fields of the source Data Integrity Field are equal to 1.</td></tr><tr><td>Application Tag F Detect = 1</td><td>All bits of the Application Tag field of the source Data Integrity Field are equal to 1.</td></tr><tr><td>Application and Reference Tag F Detect = 1</td><td>All bits of both the Application Tag and Reference Tag fields of the source Data Integrity Field are equal to 1.</td></tr></table>

# 8.3.18 DIX Generate

The DIX Generate operation, 0x17, computes the Data Integrity Field (DIF) on the source data and writes only the computed DIF for each source block to the Destination Address. The source data is not copied to the destination region.

The number of source bytes is given by Transfer Size. DIF computation is performed on each block of source data that is 512, 520, 4096, or 4104 bytes. The transfer size should be a multiple of the source block size. If the operation completes successfully, the number of bytes written to the destination is the number of source blocks multiplied by 8. There is no alignment requirement for the Source Address, but the Destination Address must be aligned to a multiple of 8. If the source and destination regions overlap, it is an error.

If the operation completes successfully, the final Reference Tag and Application Tag are written to the completion record along with a Success completion status. If the operation is partially completed due to a page fault, updated values of Reference Tag and Application Tag are written to the completion record along with the page fault information. If software corrects the fault and resumes the operation, it may copy these fields into the continuation descriptor. The Completion Record Address Valid and Request Completion Record flags must be 1 and the Completion Record Address must be valid.

See section 8.3.17, DIF Update, for a description of DIF Flags, Destination DIF Flags, and the fields in the completion record. See Appendix B for details of DIF computation.


DIX Generate Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td rowspan="3" colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td rowspan="2" colspan="4">Reserved</td><td colspan="2">DIF Flags</td><td>Dest DIF Flags</td><td></td><td>40</td></tr><tr><td colspan="2"></td><td></td><td></td><td>48</td></tr><tr><td colspan="2">Application Tag Seed</td><td colspan="2">Application Tag Mask</td><td colspan="6">Reference Tag Seed</td><td>56</td></tr></table>


DIX Generate Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td></td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="8">Unused</td><td>16</td></tr><tr><td colspan="2">Application Tag</td><td colspan="2">Application Tag Mask</td><td colspan="4">Reference Tag</td><td>24</td></tr></table>

# 8.3.19 Type Conversion

The Type operation, 0x18, reads data elements of the type specified by IData Type from the Source Address, converts each element to the type specified by OData Type and writes the result to the Destination Address. The Element Count field specifies the number of input and output data elements of the specified data type. The element size in bytes is implied by the corresponding Data Type field in the descriptor (as described in section 8.1.10). The total number of bytes to read from the Source Address is the product of Element Count and element size specified by IData Type. The total number of bytes to write to the Destination Address is the product of Element Count and element size specified by OData Type. The total number of bytes to read or write must not exceed the value specified by the WQ Maximum Transfer Size in WQCFG.

The Source and Destination Addresses must be naturally aligned to the size of the respective data type. If the source and destination regions overlap, then they must be identical (i.e. the source and destination addresses must be the same and IData Type and OData Type must be the same). IData Type and OData Type must be either both integer types or both floating point types. All combinations of integer types for IData Type and OData Type are supported. For FP types, the set of supported conversions is specified by DSACAP1 (described in 9.2.31). If IData Type and OData Type are the same, then no conversion is performed, but the actions indicated by Compute Flags are performed.

To reference an alternate address space for the source or destination or both, software may set the Use Alternate PASID flags described in Table 8-20. If both Use Alternate Source PASID and Use Alternate Destination PASID are 0, then IDPT handles are not used for either source or destination accesses. If a descriptor specifies IDPT handles, they must reference an Inter-Domain Permissions Table entry of one of the types specified in section 3.14.1.

If an IDPT Handle is specified and Window Enable and Window Mode are both 1 in the IDPT entry, the address in the descriptor and the Window Base in the IDPT entry must both be naturally aligned to the size of the corresponding data type. If no IDPT handle is specified, then an error is reported for


Type Conversion


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td colspan="2">Byte 1</td><td colspan="2">Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="5">PASID</td><td>0</td></tr><tr><td colspan="12">Completion Record Address</td><td>8</td></tr><tr><td colspan="12">Source Address</td><td>16</td></tr><tr><td colspan="12">Destination Address</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="8">Element Count</td><td>32</td></tr><tr><td colspan="2"></td><td rowspan="2" colspan="10">Reserved</td><td>40</td></tr><tr><td colspan="2"></td><td>48</td></tr><tr><td colspan="2">Destination IDPT Handle</td><td colspan="2">Source IDPT Handle</td><td colspan="2"></td><td colspan="2">Compute Flags</td><td colspan="2"></td><td>OData Type</td><td>IData Type</td><td>56</td></tr></table>


Type Conversion Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="6">Unused</td><td colspan="2">Fault IDPT Handle</td><td>16</td></tr><tr><td colspan="2"></td><td>24</td></tr></table>

overlapping buffers if they are not identical as described above. If an IDPT handle is specified, then buffer overlap checks are not done and the behavior is undefined if the buffers overlap in physical memory.

The meaning of the Result field in the completion record is described in 8.2.2.

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>17</td><td>Use Alternate Destination PASID
0: The Destination IDPT Handle field is not used.
1: The Destination IDPT Handle field references an Inter-Domain Permissions Table entry which specifies the PASID used to access the Destination Address.
This field is reserved if the Type Conversion bit in the Operations with Inter-Domain Support field in DSACAP0 is 0 (as described in section 9.2.30).</td></tr><tr><td>16</td><td>Use Alternate Source PASID
0: The Source IDPT Handle field is not used.
1: The Source IDPT Handle field references an Inter-Domain Permissions Table entry which specifies the PASID used to access the Source Address.
This field is reserved if the Type Conversion bit in the Operations with Inter-Domain Support field in DSACAP0 is 0 (as described in section 9.2.30).</td></tr></table>

Table 8-20: Type Conversion Operation-Specific Flags

# 8.3.20 Reduce

The Reduce operation, 0x19, combines the data from Source1 Address and Source2 Address and writes the result to the Destination Address. The Compute Type and Compute Flags fields specify the compute operation used to combine the source data. Refer to sections 8.1.10 and 8.1.11 for a description of the data types and compute operations. DSACAP1 indicates the set of compute operations supported by the implementation.

Compute operations are performed element-wise. The IData Type and OData Type fields specify the data type of each element in the source and destination buffers respectively. The Element Count field specifies the number of input and output data elements of the specified data type. The element size in bytes is implied by the Data Type field in the descriptor (as described in section 8.1.10).

The total number of bytes to read from each of Source1 Address and Source2 Address is the product of Element Count and element size specified by IData Type. The total number of bytes to write to the Destination Address is the product of Element Count and element size specified by OData Type. The total number of bytes to read from each source region or write to the destination region must not exceed the value specified by the WQ Maximum Transfer Size in WQCFG.

Source1 Address, Source2 Address, and Destination Address must be naturally aligned to the size of the respective data type. IData Type and OData Type must conform to the rules specified for the Type Conversion operation in section 8.3.19. If IData Type and OData Type are different, then type conversion is performed. If the destination region overlaps with a source region, then they must be identical (i.e. the corresponding source and destination addresses must be the same and IData Type and OData Type must be the same).

If the Reduce bit in the Operations with Inter-Domain Support field in DSACAP0 is 1 (as described in section 9.2.30), software is allowed to use the Inter-Domain Selector field (described in section 8.1.13) to control the address space association for each source and destination address in the descriptor. Software does this by setting the Use Inter-Domain Selector flag to 1 and specifying a non-zero value for


Reduce


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td colspan="2">Byte 1</td><td colspan="2">Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="5">PASID</td><td>0</td></tr><tr><td colspan="12">Completion Record Address</td><td>8</td></tr><tr><td colspan="12">Source1 Address</td><td>16</td></tr><tr><td colspan="12">Source2 Address</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="8">Element Count</td><td>32</td></tr><tr><td colspan="12">Destination Address</td><td>40</td></tr><tr><td colspan="12"></td><td>48</td></tr><tr><td colspan="2">IDPT Handle2</td><td colspan="2">IDPT Handle1</td><td>Inter-Domain Selector</td><td colspan="3">Compute Flags</td><td colspan="2">Compute Type</td><td>OData Type</td><td>IData Type</td><td>56</td></tr></table>


Reduce Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="6">Unused</td><td colspan="2">Fault IDPT Handle</td><td>16</td></tr><tr><td colspan="2"></td><td>24</td></tr></table>

the Inter-Domain Selector. If the selector specifies use of an IDPT handle, the handle must reference an Inter-Domain Permissions Table entry of one of the types described in section 3.14.1.

If an IDPT Handle is specified and Window Enable and Window Mode are both 1 in the IDPT entry, the address in the descriptor and the Window Base in the IDPT entry must both be naturally aligned to the size of the corresponding data type. Buffer overlap checks are performed between each pair of source and destination addresses. If an IDPT handle is not specified for either address in a pair and the addresses overlap, then an error is reported unless they are identical as described earlier. If an IDPT handle is specified for either address in a pair, then buffer overlap checks are not performed, and the behavior is undefined if the buffers overlap in physical memory. Buffer overlap checks are not performed between source addresses.

The meaning of the Result field in the completion record is described in 8.2.2.

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>23:17</td><td>Reserved: Must be 0.</td></tr><tr><td>16</td><td>Use Inter-Domain Selector
0: Inter-Domain Selector and IDPT Handle fields in the descriptor are reserved.
1: Inter-Domain Selector identifies the IDPT Handle to use for each source and destination address (as described in 8.1.13). It must be non-zero.
This field is reserved if the Reduce bit in the Operations with Inter-Domain Support field in DSACAP0 is 0 (as described in section 9.2.30).</td></tr></table>

Table 8-21: Reduce Operation-Specific Flags

# 8.3.21 Reduce with Dualcast

The Reduce with Dualcast operation, 0x1A, behaves identically to the Reduce operation, except that the results are written to both Destination1 and Destination2. Source1, Source2, Destination1 and Destination2 Address must be naturally aligned to the size of the respective data type. Additionally, bits 11:0 of the two destination addresses must be the same. If either destination region overlaps with a source region, then they must be identical (i.e., the corresponding source and destination addresses must be the same and IData Type and OData Type must be the same).

The Reduce with Dualcast bit in the Operations with Inter-Domain Support field in DSACAP0 indicates whether software is allowed to use the Inter-Domain Selector field (described in 8.1.13) to control the address space association for each source and destination address in the descriptor. Other details pertaining to inter-domain operation are similar to the Reduce operation in section 8.3.20. Buffer overlap checks are performed between each pair of source and destination addresses, and between destination addresses. If an IDPT handle is not specified for either address in a pair and the addresses overlap, then an error is reported unless they are identical as described earlier. If an IDPT handle is specified for either address in a pair, then buffer overlap checks are not performed, and the behavior is undefined if the buffers overlap in physical memory. Buffer overlap checks are not performed between source addresses.

The use of TC selector flags with this operation is described in section 8.1.3. Additionally, software can use the Destination2 TC Selector flag to select a traffic class value corresponding to Destination2 Address.

The meaning of the Result field in the completion record is described in 8.2.2.


Reduce with Dualcast


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td colspan="2">Byte 1</td><td colspan="2">Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="5">PASID</td><td>0</td></tr><tr><td colspan="12">Completion Record Address</td><td>8</td></tr><tr><td colspan="12">Source1 Address</td><td>16</td></tr><tr><td colspan="12">Source2 Address</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="8">Element Count</td><td>32</td></tr><tr><td colspan="12">Destination1 Address</td><td>40</td></tr><tr><td colspan="12">Destination2 Address</td><td>48</td></tr><tr><td colspan="2">IDPT Handle2</td><td colspan="2">IDPT Handle1</td><td>Inter-Domain Selector</td><td colspan="3">Compute Flags</td><td>Compute Type</td><td>OData Type</td><td colspan="2">IDData Type</td><td>56</td></tr></table>


Reduce with Dualcast Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="6" rowspan="2">Unused</td><td colspan="2">Fault IDPT Handle</td><td>16</td></tr><tr><td colspan="2"></td><td>24</td></tr><tr><td colspan="2">Bits</td><td>Description</td></tr><tr><td>23:18</td><td colspan="8">Reserved: Must be 0.</td></tr><tr><td>17</td><td colspan="8">Destination2 TC Selector
For writes to Destination2 Address, this field selects one of the two Traffic Class values in the Group Configuration Register corresponding to the WQ that the descriptor was submitted to.
0: Use TC-A in the Group Configuration Register.
1: Use TC-B in the Group Configuration Register.</td></tr><tr><td>16</td><td colspan="8">Use Inter-Domain Selector
0: Inter-Domain Selector and IDPT Handle fields in the descriptor are reserved.
1: Inter-Domain Selector identifies the IDPT Handle to use for each source and destination address (as described in 8.1.13). It must be non-zero.
This field is reserved if the Reduce with Dualcast bit in the Operations with Inter-Domain Support field in DSACAP0 is 0 (as described in section 9.2.30).</td></tr></table>

Table 8-22: Reduce with Dualcast Operation-Specific Flags

# 8.3.22 Gather Reduce

The Gather Reduce operation, 0x1B, combines a set of input vectors of a specified size to produce a single output vector of the same size as shown in Figure 8-2. The list of source memory locations identifying the input vectors is specified by an SGL as described in section 8.1.14. The SGL format is specified by the SGL Format field. The output vector is written to the Destination Address. The Compute Type and Compute Flags fields specify the compute operation used to combine the input vectors element-wise. Refer to section 8.1.11 and 8.1.12 for details of the compute operations and flags. The IData Type and OData Type fields specify the data type of each element in the source and destination buffers respectively, as described in section 8.1.10. The Element Count field specifies the number of input and output data elements of the specified data type. The source addresses corresponding to each list entry, and the destination address must be naturally aligned to the size of the respective data type.

IData Type and OData Type must conform to the rules specified for the Type Conversion operation in section 8.3.19. If IData Type and OData Type are different, then type conversion is performed.

![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/e31028000081e21a12dc27f872b6da06825159df2701d21d95a5b9f124c8de4b.jpg)



Figure 8-2: Illustration of Gather Reduce operation



Gather Reduce


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td colspan="2">Byte 1</td><td colspan="2">Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="5">PASID</td><td>0</td></tr><tr><td colspan="12">Completion Record Address</td><td>8</td></tr><tr><td colspan="12">Scatter-Gather List Address</td><td>16</td></tr><tr><td colspan="12">Destination Address</td><td>24</td></tr><tr><td rowspan="2" colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="8">Element Count</td><td>32</td></tr><tr><td colspan="2">SGL Size</td><td colspan="8"></td><td>40</td></tr><tr><td colspan="12">Base Address</td><td>48</td></tr><tr><td colspan="4"></td><td>SGL Format</td><td></td><td colspan="2">Compute Flags</td><td colspan="2">Compute Type</td><td>OData Type</td><td>IData Type</td><td>56</td></tr></table>


Gather Reduce Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="2"></td><td colspan="2">SGL Processed</td><td colspan="4"></td><td>16</td></tr><tr><td colspan="6">Unused</td><td>24</td></tr></table>

For the example in Figure 8-2, each element in the output vector may be expressed in terms of the following equation:

$\mathsf{o_n} = \mathsf{a_n} \oplus \mathsf{b_n} \oplus \mathsf{c_n}$  (where  $\oplus$  represents the compute operation performed)

The number of bytes to read from each source address is referred to as the input block size

input block size = Element Count * element size (corresponding to IData Type)

The number of bytes to write to the destination memory is referred to as the output block size

output block size = Element Count * element size (corresponding to OData Type)

The total number of bytes written to the destination address is equal to the output block size. Both the input block size and output block size must be less than Maximum Supported Gather Reduce Block Size in DSACAP0.

If all the list entries were processed successfully, the Status field of the completion record indicates a success status. Bytes Completed is always 0 for this operation.<sup>1</sup> If a page fault occurs, the operation must be restarted from the beginning. The value of the SGL Processed field in the completion record indicates the number of SGL entries processed.

If the destination region overlaps with a source region, the data written is undefined.

The meaning of the Result field in the completion record is described in 8.2.2.

# 8.3.23 Gather Copy

The Gather Copy operation, 0x1C, reads data from one or more source memory locations specified by a Scatter-Gather List and writes it to sequential locations at the Destination Address. See section 8.1.14 for details on the SGL formats.

The Transfer Size field must be equal to the total number of bytes represented by the SGL entries. The number of bytes to copy from each source location to the corresponding destination location is specified in terms of Element Count and data type of each element and is the product of Element Count and element size. This is referred to as the block size.

block size = Element Count * element size

The element size in bytes is specified by the Data Type field in the descriptor (as described in section 8.1.10). There are no alignment requirements for the memory addresses.

To reference an alternate address space for the source or destination or both, software may set one of the Use Alternate PASID flags described in Table 8-23. If both Use Alternate Source PASID and Use Alternate Destination PASID are 0, then IDPT handles are not used for either source or destination accesses.

If all the SGL entries were processed and the data copied successfully, the Status field of the completion record indicates Success and SGL Completed and Bytes Completed are 0. If the Status field indicates a page fault, the SGL Completed field of the completion record contains the number of SGL entries that were completely processed before the fault occurred. Bytes Completed indicates the total number of bytes processed before the fault occurred and may include a partial SGL entry if the fault was detected part-way through the processing on an SGL entry. It is implementation dependent whether Bytes Completed corresponds to fully processed SGL entries or includes a partial SGL entry.

If the destination overlaps the SGL, an error is reported and the operation is not performed. If the destination overlaps any of the source buffers, the data written is undefined.


Gather Copy


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source SGL Address</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td rowspan="2" colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="2">SGL Size</td><td colspan="6">Element Count</td><td>40</td></tr><tr><td colspan="10">Base Address</td><td>48</td></tr><tr><td colspan="2">Destination IDPT Handle</td><td colspan="2">Source IDPT Handle</td><td>SGL Format</td><td colspan="4"></td><td>Data Type</td><td>56</td></tr></table>


Gather Copy Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Unused</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="2"></td><td colspan="2">SGL Completed</td><td colspan="2"></td><td colspan="2">Fault IDPT Handle</td><td>16</td></tr><tr><td colspan="8">Unused</td><td>24</td></tr><tr><td colspan="8">Bits</td><td>Description</td></tr><tr><td>23:18</td><td colspan="8">Reserved: Must be 0.</td></tr><tr><td>17</td><td colspan="8">Use Alternate Destination PASID
0: An IDPT Handle is not used for destination writes.
1: An IDPT Handle is used for destination writes.
This field is reserved if the Gather Copy bit in the Operations with Inter-Domain Support field in DSACAP0 is 0 (as described in section 9.2.30).</td></tr><tr><td>16</td><td colspan="8">Use Alternate Source PASID
0: An IDPT Handle is not used for source reads.
1: An IDPT Handle is used for source reads.
This field is reserved if the Gather Copy bit in the Operations with Inter-Domain Support field in DSACAP0 is 0 (as described in section 9.2.30).</td></tr></table>

Table 8-23: Gather Copy Operation-Specific Flags

# 8.3.24 Scatter Copy

The Scatter Copy operation, 0xD, reads data from sequential locations at the Source Address and writes it to one or more destination memory locations specified by a Scatter-Gather List. Refer to section 8.1.14 for details on the SGL and related flags.

The Transfer Size field must be equal to the total number of bytes represented by the SGL entries. The number of bytes to copy from each source location to the corresponding destination location is specified in terms of Element Count and data type of each element and is the product of Element Count and element size. This is referred to as the block size.

block size = Element Count * element size

The element size in bytes is specified by the Data Type field in the descriptor (as described in section 8.1.10). There are no alignment requirements for the memory addresses.

To reference an alternate address space for the source or destination or both, software may set one of the Use Alternate PASID flags described in Table 8-24. If both Use Alternate Source PASID and Use Alternate Destination PASID are 0, then IDPT handles are not used for either source or destination accesses.

If all the SGL entries were processed and the data copied successfully, the Status field of the completion record indicates Success and SGL Completed and Bytes Completed are 0. If the Status field indicates a page fault, the SGL Completed field of the completion record contains the number of SGL entries that were completely processed before the fault occurred. Bytes Completed indicates the total number of bytes processed before the fault occurred and may include a partial SGL entry if the fault was detected part-way through the processing on an SGL entry. It is implementation dependent whether Bytes Completed corresponds to fully processed SGL entries or includes a partial SGL entry.

If any part of the source overlaps any part of the destination or any of the destination buffers overlap each other, the data written is undefined. If any of the destination buffers overlap the SGL, the behavior is undefined and the result produced is implementation-specific.


Scatter Copy


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Destination SGL Address</td><td>24</td></tr><tr><td rowspan="2" colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="2">SGL Size</td><td colspan="6">Element Count</td><td>40</td></tr><tr><td colspan="10">Base Address</td><td>48</td></tr><tr><td colspan="2">Destination IDPT Handle</td><td colspan="2">Source IDPT Handle</td><td>SGL Format</td><td colspan="4"></td><td>Data Type</td><td>56</td></tr></table>


Scatter Copy Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Unused</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="2"></td><td colspan="2">SGL Completed</td><td colspan="2"></td><td colspan="2">Fault IDPT Handle</td><td>16</td></tr><tr><td colspan="8">Unused</td><td>24</td></tr><tr><td colspan="8">Bits</td><td>Description</td></tr><tr><td>23:18</td><td colspan="8">Reserved: Must be 0.</td></tr><tr><td>17</td><td colspan="8">Use Alternate Destination PASID
0: An IDPT Handle is not used for destination writes.
1: An IDPT Handle is used for destination writes.
This field is reserved if the Scatter Copy bit in the Operations with Inter-Domain Support field in DSACAP0 is 0 (as described in section 9.2.30).</td></tr><tr><td>16</td><td colspan="8">Use Alternate Source PASID
0: An IDPT Handle is not used for source reads.
1: An IDPT Handle is used for source reads.
This field is reserved if the Scatter Copy bit in the Operations with Inter-Domain Support field in DSACAP0 is 0 (as described in section 9.2.30).</td></tr></table>

Table 8-24: Scatter Copy Operation-Specific Flags

# 8.3.25 Scatter Fill

The Scatter Fill operation, 0x1E, fills memory at the destination memory locations specified by a Scatter-Gather List with the value in the pattern field. Refer to section 8.1.14 for details on the SGL and related flags. The description of the pattern and other software requirements are identical to the Fill operation described in section 8.3.5, except that the pattern size is always 8 bytes.

The Transfer Size field must be equal to the total number of bytes represented by the SGL entries. The number of bytes to write to each destination location is specified in terms of Element Count and data type of each element and is the product of Element Count and element size. This is referred to as the block size.

block size = Element Count * element size

The element size in bytes is specified by the Data Type field in the descriptor (as described in section 8.1.10). There are no alignment requirements for the memory addresses.

The Use Alternate Destination PASID flag specifies whether an IDPT Handle is used for destination writes.

The description of page fault reporting and completion record format for Scatter Fill are identical to Scatter Copy (as described in section 8.3.24). If any of the destination buffers overlap each other, the data written is undefined. If any of the destination buffers overlap the SGL, the behavior is undefined and the result produced is implementation-specific.


Scatter Fill


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Pattern</td><td>16</td></tr><tr><td colspan="10">Destination SGL Address</td><td>24</td></tr><tr><td rowspan="2" colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="2">SGL Size</td><td colspan="6">Element Count</td><td>40</td></tr><tr><td colspan="10">Base Address</td><td>48</td></tr><tr><td colspan="2">Destination IDPT Handle</td><td colspan="2"></td><td>SGL Format</td><td colspan="4"></td><td>Data Type</td><td>56</td></tr></table>


Scatter Fill Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Unused</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td colspan="2"></td><td colspan="2">SGL Completed</td><td colspan="2"></td><td colspan="2">Fault IDPT Handle</td><td>16</td></tr><tr><td colspan="8">Unused</td><td>24</td></tr><tr><td colspan="8">Bits</td><td>Description</td></tr><tr><td>23:19</td><td colspan="8">Reserved: Must be 0.</td></tr><tr><td>18</td><td colspan="8">Pattern Size
This field must be 0. Pattern size is 8B and specified in the Pattern field.</td></tr><tr><td>17</td><td colspan="8">Use Alternate Destination PASID
0: An IDPT Handle is not used for destination writes.
1: An IDPT Handle is used for destination writes.
This field is reserved if the Scatter Fill bit in the Operations with Inter-Domain Support field in DSACAP0 is 0 (as described in section 9.2.30).</td></tr><tr><td>16</td><td colspan="8">Reserved: Must be 0.</td></tr></table>

Table 8-25: Scatter Fill Operation-Specific Flags

# 8.3.26 Cache Flush

The Cache Flush operation, 0x20, flushes the processor caches at the Destination Address. The number of bytes flushed is given by Transfer Size. The transfer size does not need to be a multiple of the cache line size. There are no alignment requirements for the destination address or the transfer size. Any cache line that is partially covered by the destination region is flushed.

Supported encodings of the cache control flags are described in Table 8-4. Combinations not specifically listed as supported for the Cache Flush operation are reserved.


Cache Flush Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Reserved</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td rowspan="4" colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td rowspan="3" colspan="8">Reserved</td><td>40</td></tr><tr><td>48</td></tr><tr><td>56</td></tr></table>


Cache Flush Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td></td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="8">Unused</td><td>16</td></tr><tr><td>24</td></tr></table>

# 8.3.27 Update Window

The Update Window operation, 0x21, atomically modifies attributes of the memory window associated with the specified Inter-Domain Permissions Table entry. The descriptor PASID must match the access PASID in the entry referenced by the handle, and the Allow Update bit in the entry must be 1. There are no alignment requirements for the Window Base Address or the Window Size fields. If the Window Enable field in Window Flags is 1, the sum of Window Base Address and Window Size in the descriptor must be less than or equal to  $2^{64}$ . If Window Enable is 0, then the Window Mode, Window Base Address, and Window Size fields in the descriptor must be 0.

As described in section 3.14.5 an implicit drain is performed to flush out any in-flight descriptors that are still using pre-update window attributes. Software can use the Suppress Drain flag to avoid the implicit drain if necessary.

An Update Window descriptor may not be included in a batch; it is treated as an unsupported operation type.


Update Window Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Window Base Address</td><td>16</td></tr><tr><td colspan="10">Window Size</td><td>24</td></tr><tr><td rowspan="3" colspan="2"></td><td rowspan="3" colspan="2">Completion Interrupt Handle</td><td rowspan="3" colspan="6"></td><td>32</td></tr><tr><td>40</td></tr><tr><td>48</td></tr><tr><td colspan="2">IDPT Handle</td><td colspan="2">Window Flags</td><td colspan="6"></td><td>56</td></tr></table>


Update Window Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="7" rowspan="4">Unused</td><td>Status</td><td>0</td></tr><tr><td></td><td>8</td></tr><tr><td></td><td>16</td></tr><tr><td></td><td>24</td></tr><tr><td>Bits</td><td>Description</td></tr><tr><td>7:4</td><td>Reserved: Must be 0.</td></tr><tr><td>3</td><td>Window Mode
0: Window operates in Address Mode.
1: Window operates in Offset Mode.
See section 3.14.3 for details.
This field is reserved if Window Enable is 0 or if Offset Mode Support in IDCAP is 0.</td></tr><tr><td>2</td><td>Window Enable
0: The window address range checks are disabled, and hardware will not perform range checks on the incoming address.
1: The window address range checks are enabled, and hardware will perform range checks on the incoming address based on the window mode.</td></tr><tr><td>1</td><td>Write Permissions
0: Disallows memory write using this entry.
1: Allows memory writes using this entry.</td></tr><tr><td>0</td><td>Read Permissions
0: Disallows memory reads using this entry.
1: Allows memory reads using this entry.</td></tr></table>


Table 8-26: Update Window - Window Flags


<table><tr><td>Bits</td><td>Description</td></tr><tr><td>23:17</td><td>Reserved: Must be 0.</td></tr><tr><td>16</td><td>Suppress Drain
0: Drain any descriptors using prior values of the IDPTE fields modified by this descriptor.
1: No drain is performed.
This field is reserved if Update Window Suppress Drain Support in IDCAP is 0.</td></tr></table>

Table 8-27: Update Window Operation-Specific Flags

# 8.3.28 Inter-Domain Copy

The Inter-Domain Copy operation, 0x23, copies memory from the Source Address to the Destination Address. At least one of Source or Destination Address must be qualified with an IDPT handle and it must reference an Inter-Domain Permissions Table entry of one of the types specified in section 3.14.1. Software does this by setting at least one of the flags described in the table below.

The number of bytes copied is specified by Transfer Size. There are no alignment requirements for the memory addresses or the transfer size.

If the source and destination regions overlap in physical memory, the behavior is undefined.

See section 3.14.4 for information on handling failures in inter-domain operations.


Inter-Domain Copy Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td rowspan="2" colspan="2"></td><td rowspan="2" colspan="8">Reserved</td><td>40</td></tr><tr><td>48</td></tr><tr><td colspan="2">Destination IDPT Handle</td><td colspan="2">Source IDPT Handle</td><td colspan="6"></td><td>56</td></tr></table>


Inter-Domain Copy Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td></td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="6">Unused</td><td colspan="2">Fault IDPT Handle</td><td>16</td></tr><tr><td colspan="2"></td><td>24</td></tr></table>

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>23:18</td><td>Reserved: Must be 0.</td></tr><tr><td>17</td><td>Use Alternate Destination PASID
0: The Destination IDPT Handle field is not used.
1: The Destination IDPT Handle field references an Inter-Domain Permissions Table entry which specifies the PASID used to access the Destination Address.</td></tr><tr><td>16</td><td>Use Alternate Source PASID
0: The Source IDPT Handle field is not used.
1: The Source IDPT Handle field references an Inter-Domain Permissions Table entry which specifies the PASID used to access the Source Address.</td></tr></table>

Table 8-28: Inter-Domain Copy Operation-Specific Flags

# 8.3.29 Inter-Domain Fill

The Inter-Domain Fill operation, 0x24, fills memory at the Destination Address with the value in the pattern field. A Destination IDPT handle must be specified, and it must reference an Inter-Domain Permissions Table entry of one of the types specified in section 3.14.1.

The description of the pattern and other software requirements are identical to the Fill operation described in section 8.3.5.

See section 3.14.4 for information on handling failures in inter-domain operations.


Inter-Domain Fill Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Pattern Lower</td><td>16</td></tr><tr><td colspan="10">Destination Address</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="10">Pattern Upper</td><td>40</td></tr><tr><td colspan="10"></td><td>48</td></tr><tr><td colspan="2">Destination IDPT Handle</td><td colspan="8">Reserved</td><td>56</td></tr></table>


Inter-Domain Fill Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td></td><td>Fault Info</td><td></td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="6">Unused</td><td colspan="2">Fault IDPT Handle</td><td>16</td></tr><tr><td colspan="2"></td><td>24</td></tr></table>

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>23:19</td><td>Reserved: Must be 0.</td></tr><tr><td>18</td><td>Pattern Size
0: Pattern size is 8B and specified in the Pattern Lower field. The Pattern Upper field is reserved.
1: Pattern size is 16B and specified by the Pattern Lower and Pattern Upper fields. This field must be 0 if Fillló Support in GENCAP is 0.</td></tr><tr><td>17</td><td>Use Alternate Destination PASID
This field must be 1. The Destination IDPT Handle field references an Inter-Domain Permissions Table entry which specifies the PASID used to access the Destination Address.</td></tr><tr><td>16</td><td>Reserved: Must be 0.</td></tr></table>

Table 8-29: Inter-Domain Fill Operation-Specific Flags

# 8.3.30 Inter-Domain Compare

The Inter-Domain Compare operation, 0x25, compares memory at Source1 Address with memory at Source2 Address. At least one of Source1 or Source2 Address must be qualified with an IDPT handle and it must reference an Inter-Domain Permissions Table entry of one of the types specified in section 3.14.1. Software does this by setting at least one of the flags described in the table below.

The reporting of results from this operation is identical to the Compare operation described in section 8.3.6.

See section 3.14.4 for information on handling failures in inter-domain operations.


Inter-Domain Compare Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source1 Address</td><td>16</td></tr><tr><td colspan="10">Source2 Address</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="4"></td><td rowspan="2" colspan="5">Reserved</td><td>Expected Result</td><td>40</td></tr><tr><td colspan="4"></td><td></td><td>48</td></tr><tr><td colspan="2">Source2 IDPT Handle</td><td colspan="2">Source1 IDPT Handle</td><td colspan="6"></td><td>56</td></tr></table>


Inter-Domain Compare Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td>Unused</td><td>Fault Info</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="6">Unused</td><td colspan="2">Fault IDPT Handle</td><td>16</td></tr><tr><td colspan="2"></td><td>24</td></tr></table>

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>23:18</td><td>Reserved: Must be 0.</td></tr><tr><td>17</td><td>Use Alternate Source2 PASID
0: The Source2 IDPT Handle field is not used.
1: The Source2 IDPT Handle field references an Inter-Domain Permissions Table entry which specifies the PASID used to access Source2 Address.</td></tr><tr><td>16</td><td>Use Alternate Source1 PASID
0: The Source1 IDPT Handle field is not used.
1: The Source1 IDPT Handle field references an Inter-Domain Permissions Table entry which specifies the PASID used to access Source1 Address.</td></tr></table>

Table 8-30: Inter-Domain Compare Operation-Specific Flags

# 8.3.31 Inter-Domain Compare Pattern

The Inter-Domain Compare Pattern operation, 0x26, compares memory at Source Address with the value in the pattern field. The Source Address must be qualified with an IDPT handle and it must reference an Inter-Domain Permissions Table entry of one of the types specified in section 3.14.1.

The description of the pattern in the descriptor and the reporting of results from this operation are identical to the Compare Pattern operation described in section 8.3.7. The behavior of Check Result and Expected Result are identical to the Compare operation in section 8.3.6.

See section 3.14.4 for information on handling failures in inter-domain operations.


Inter-Domain Compare Pattern Descriptor


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td colspan="2">Byte 3</td><td colspan="2">Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td>Operation</td><td colspan="3">Flags</td><td>Priv</td><td colspan="2">Reserved</td><td colspan="3">PASID</td><td>0</td></tr><tr><td colspan="10">Completion Record Address</td><td>8</td></tr><tr><td colspan="10">Source Address</td><td>16</td></tr><tr><td colspan="10">Pattern</td><td>24</td></tr><tr><td colspan="2"></td><td colspan="2">Completion Interrupt Handle</td><td colspan="6">Transfer Size</td><td>32</td></tr><tr><td colspan="2"></td><td rowspan="2" colspan="6">Reserved</td><td rowspan="2" colspan="2">Expected Result</td><td>40</td></tr><tr><td colspan="2"></td><td>48</td></tr><tr><td colspan="2"></td><td colspan="2">Source IDPT Handle</td><td colspan="6"></td><td>56</td></tr></table>


Inter-Domain Compare Pattern Completion Record


<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td colspan="4">Bytes Completed</td><td colspan="2">Unused</td><td>Result</td><td>Status</td><td>0</td></tr><tr><td colspan="8">Fault Address</td><td>8</td></tr><tr><td rowspan="2" colspan="6">Unused</td><td colspan="2">Fault IDPT Handle</td><td>16</td></tr><tr><td colspan="2"></td><td>24</td></tr></table>

<table><tr><td>Bits</td><td>Description</td></tr><tr><td>23:17</td><td>Reserved: Must be 0.</td></tr><tr><td>16</td><td>Use Alternate Source PASIDThis field must be 1. The Source IDPT Handle field references an Inter-DomainPermissions Table entry which specifies the PASID used to access the Source Address.</td></tr></table>

Table 8-31: Inter-Domain Compare Pattern Operation-Specific Flags

S

# 9 Register Descriptions

The programming interface for the Intel Data Streaming Accelerator consists of PCI configuration registers and MMIO registers, which include configuration and control registers and work submission portals. The base addresses for the MMIO registers and portals are specified by two Base Address Registers (BARs) in PCI config space.

PCI config space accesses must be performed as aligned 1-, 2-, or 4-byte accesses. See the PCI Express Base Specification listed in section 1.2 for rules on accessing unimplemented registers and reserved bits in PCI config space.

MMIO space accesses to the BARO region (capability, configuration, and status registers) must be performed as aligned 1-, 2-, 4- or 8-byte accesses. Software may use 8-byte accesses for any registers, including accessing two adjacent 32-bit registers with a single 8-byte access.

MMIO space accesses to the BAR2 region must be 64-byte writes, as described in section 9.3.

This chapter and Appendix C use the following abbreviations for register attributes.

<table><tr><td>Attribute</td><td>Abbreviation</td><td>Description</td></tr><tr><td>Read/Write</td><td>RW</td><td>The field can be read and written by software. The value read matches the value last written. Bits that are reserved or not supported by an implementation may be hardwired to 0.</td></tr><tr><td>Read/Write/Lock</td><td>RWL</td><td>The field is read-write at some times and read-only at other times. The specification of each register or field describes when it is read-only. The value read matches the value last written while the field was writeable. Bits that are reserved or not supported by an implementation may be hardwired to 0.</td></tr><tr><td>Read Only</td><td>RO</td><td>The field is set by the hardware and software can only read it. In some cases, the field has a fixed value (e.g., in a capability register), and in some cases the field reports status that can change during device operation. Writes to the field have no effect.</td></tr><tr><td>Write Only</td><td>WO</td><td>The field is only writeable by software. Reads return 0.</td></tr><tr><td>Read/Write-1-to-Clear</td><td>RWIC</td><td>The field can be read or cleared by software. To clear an RWIC bit, software writes a one to it. Writing a zero to an RWIC bit has no effect.</td></tr><tr><td>Read Only Sticky</td><td>ROS</td><td>The field reports status and software can only read it. Writes to the register have no effect. The field is not cleared on reset.</td></tr><tr><td>Read/Write-1-to-Clear Sticky</td><td>RWICS</td><td>The field behaves the same as RWIC except that it is not cleared on reset.</td></tr><tr><td>Reserved</td><td>RSVD</td><td>Read as 0. Ignored on writes. Software must write 0 for compatibility with future expansion.</td></tr><tr><td>Read/Write/Volatile</td><td>RWV</td><td>The field can be read and written by software. The value may be changed by hardware, so the value read may not match the last value written.</td></tr><tr><td>Read/Write/Lock/Volatile</td><td>RWLV</td><td>The field is read-write at some times and read-only at other times. The specification of each register or field describes when it is read-only. The value may be changed by hardware, so the value read may not match the last value written.</td></tr></table>

Table 9-1: Register Attributes

# 9.1 PCI Configuration Space Registers

This section provides Intel DSA specific details about some of the PCI configuration registers. See Appendix C and the PCI Express specification listed in section 1.2 for a complete specification of these registers.

# 9.1.1 Base Address Registers (BAR)

Intel DSA PCI configuration space implements two 64-bit BARs.

# 9.1.1.1 BARO (Device Control Registers)

BARO is a 64-bit BAR that contains the physical base address of device control registers. These registers provide information about device capabilities, controls to configure and enable the device, and device status. These registers are described in detail in the following sections. The size of the BARO region depends on the specific device implementation, and is at least 64KB.

# 9.1.1.2 BAR2(Portals)

BAR2 is a 64-bit BAR that contains the physical base address of the portals that are used to submit descriptors to the device. Each portal is 64 bytes in size and is located on a separate 4 KB page. This allows the portals to be independently mapped into different address spaces using CPU page tables.

There are 4 portals per WQ, as described in section 3.3. So, for example, if the device supports 8 WQs, the size of BAR2 would be  $8 \times 4 \times 4$  KB = 128 KB. If the size is not a power of two, the total size of BAR2 is rounded up to the next power of two.

Any write to an address within the BAR2 region that does not correspond to a WQ portal is ignored; for DMWr, a Retry response is returned. Any read operation to the BAR2 address space returns either 0x00 or 0xFF for all bytes.

# 9.1.2 MSI-X Capability

MSI-X is the only PCI Express interrupt capability that Intel DSA provides. It does not implement legacy PCI interrupts or MSI. Details of this register structure are in the PCI Express specification. See section 3.7 for information on how the MSI-X table is used.

# 9.1.3 Address Translation Capabilities

Three PCI Express capabilities control address translation. If any of these capabilities are changed by software while the device is not Disabled, the device enters the Halt state and an error is reported in the Software Error register.

<table><tr><td>PASID</td><td>ATS</td><td>PRS</td><td>Operation</td></tr><tr><td>1</td><td>1</td><td>1</td><td>Addresses are translated with or without PASID, depending on the work queue configuration. (See section 9.2.24.) Recoverable page faults are supported. This is the recommended mode. This mode must be used to allow user-mode access to the device or to allow sharing among multiple guests in a virtualized system.</td></tr><tr><td>0</td><td>1</td><td>0</td><td>Addresses are translated using the BDF of the device. PASID is not used. Translation failures are not recoverable. This mode may be used when address translation is enabled in the IOMMU but the device is only used by the kernel or by a single guest kernel in a virtualized platform.</td></tr><tr><td>0</td><td>0</td><td>0</td><td>All memory accesses are Untranslated Accesses without PASID. The Address Translation Cache is not used. This mode is recommended only when IOMMU address translation is disabled.</td></tr><tr><td>1</td><td>0</td><td>0</td><td>All memory accesses are Untranslated Accesses, with or without PASID, depending on the WQ configuration. The Address Translation Cache is not used.</td></tr><tr><td>1</td><td>1</td><td>0</td><td>Addresses are translated with or without PASID, depending on the WQ configuration. Intel DSA does not perform recovery from address translation failures, but it provides information in the completion record or event log to allow software to recover from page faults.</td></tr><tr><td>0</td><td>1</td><td>1</td><td>Addresses are translated using the BDF of the device. PASID is not used. Recoverable page faults are supported.</td></tr><tr><td>0</td><td>0</td><td>1</td><td rowspan="2">Page requests are never generated when ATS is disabled, so these modes are not useful; PRS Enable is ignored.</td></tr><tr><td>1</td><td>0</td><td>1</td></tr></table>

Table 9-2: Address Translation Modes

# 9.1.3.1 PASID Capability

Software configures the PASID capability to control whether the device uses PASID to perform address translation. If PASID is disabled, shared virtual memory (SVM) is not supported, only dedicated WQs (DWQs) may be used, and the device cannot be shared across multiple VMs. PASID must always be enabled to use shared WQs (SWQs). If PASID is enabled, address translation is performed using PASID according to the IOMMU configuration.

# 9.1.3.2 ATS Capability

Software configures the ATS capability to control whether the device should translate addresses before performing memory accesses. If address translation is enabled in the IOMMU, enabling ATS in the device will generally improve system performance. If the device ATS capability is enabled and if WQ ATS Support in WQCAP is 1, software can optionally disable the use of ATS independently for each WQ by setting WQ ATS Disable in WQCFG (described in section 9.2.24). If address translation is not enabled in the IOMMU, ATS must be disabled. If ATS is disabled, all memory accesses are performed using Untranslated Accesses.

# 9.1.3.3 PRS Capability

Software configures the PRS capability to control whether the device can request a page when an address translation fails. If the device PRS capability is enabled and if WQ PRS Support in WQCAP is 1, software can optionally disable the use of PRS independently for each WQ by setting WQ PRS Disable in WQCFG (described in section 9.2.24). When ATS is enabled and PRS is not enabled, Intel DSA provides information in the completion record or event log to allow software to recover from page faults.

# 9.1.4 VC Capability

If the PCIe VC capability is present, software configures the TC/VC mapping in the PCI Express VC capability to control the mapping of different Traffic Classes to the corresponding platform and internal I/O fabric resources. Use of traffic classes is described in more detail in section 4.2.

# 9.2 Configuration and Control Registers (BAR0)

<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td rowspan="2" colspan="4"></td><td rowspan="2" colspan="4">Version</td><td>00h</td></tr><tr><td>08h</td></tr><tr><td colspan="8">General Capabilities</td><td>10h</td></tr><tr><td colspan="8"></td><td>18h</td></tr><tr><td colspan="8">Work Queue Capabilities</td><td>20h</td></tr><tr><td colspan="8"></td><td>28h</td></tr><tr><td colspan="8">Group Capabilities</td><td>30h</td></tr><tr><td colspan="8">Engine Capabilities</td><td>38h</td></tr><tr><td rowspan="4" colspan="8">Operations Capabilities</td><td>40h</td></tr><tr><td>48h</td></tr><tr><td>50h</td></tr><tr><td>58h</td></tr><tr><td rowspan="2" colspan="8">Table Offsets</td><td>60h</td></tr><tr><td>68h</td></tr><tr><td rowspan="2" colspan="8"></td><td>70h</td></tr><tr><td>78h</td></tr><tr><td rowspan="8" colspan="4"></td><td colspan="4">General Configuration</td><td>80h</td></tr><tr><td colspan="4">General Control</td><td>88h</td></tr><tr><td colspan="4">General Status</td><td>90h</td></tr><tr><td colspan="4">Interrupt Cause</td><td>98h</td></tr><tr><td colspan="4">Command</td><td>A0h</td></tr><tr><td colspan="4">Command Status</td><td>A8h</td></tr><tr><td colspan="4">Command Capabilities</td><td>B0h</td></tr><tr><td colspan="4"></td><td>B8h</td></tr><tr><td rowspan="4" colspan="8">Software Error</td><td>C0h</td></tr><tr><td>C8h</td></tr><tr><td>D0h</td></tr><tr><td>D8h</td></tr><tr><td rowspan="2" colspan="8">Event Log Configuration</td><td>E0h</td></tr><tr><td>E8h</td></tr><tr><td colspan="8">Event Log Status</td><td>F0h</td></tr><tr><td colspan="8"></td><td>F8h</td></tr><tr><td colspan="8">Inter-Domain Capabilities</td><td>100h</td></tr><tr><td colspan="4"></td><td colspan="4">Inter-Domain Bitmap Register</td><td>108h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td rowspan="3" colspan="8">DSA Capabilities</td><td>180h</td></tr><tr><td>188h</td></tr><tr><td>190h</td></tr></table>

Continued on the next page.

Figure 9-1: MMIO Register Map

<table><tr><td>Byte 7</td><td>Byte 6</td><td>Byte 5</td><td>Byte 4</td><td>Byte 3</td><td>Byte 2</td><td>Byte 1</td><td>Byte 0</td><td>bytes</td></tr><tr><td rowspan="2" colspan="4"></td><td rowspan="2" colspan="4">MSI-X Permissions Table1</td><td>400h</td></tr><tr><td>408h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td rowspan="2" colspan="8">Group Configuration Table1</td><td>600h</td></tr><tr><td>640h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td rowspan="2" colspan="8">Work Queue Configuration Table1</td><td>800h</td></tr><tr><td>840h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td rowspan="2" colspan="8">Performance Monitoring Registers1</td><td>2000h</td></tr><tr><td>2010h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td rowspan="2" colspan="8">MSI-X Table2</td><td>4000h</td></tr><tr><td>4010h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td colspan="8">MSI-X Pending Bit Array2</td><td>5000h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td rowspan="2" colspan="8">IMS Table1</td><td>8000h</td></tr><tr><td>8010h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td rowspan="2" colspan="8">Inter-Domain Permissions Table1</td><td>9000h</td></tr><tr><td>9020h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td colspan="8">Admin Command Parameters</td><td>E000h</td></tr><tr><td colspan="8"></td><td>E008h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td colspan="8">Dummy Portal</td><td>F000h</td></tr><tr><td colspan="8"></td><td></td></tr><tr><td colspan="8">FFF8h</td><td></td></tr></table>


1 The offset shown is an example. The actual offset of this table is given in the Table Offsets register.



2 The offset shown is an example. The actual offset of this table is given in the PCIe MSI-X capability.



The initial values of MMIO-space registers are as follows:


<table><tr><td rowspan="2">Register</td><td colspan="4">Initial Value</td></tr><tr><td>Power-On Reset</td><td>Warm Reset</td><td>Function-Level Reset</td><td>Software Reset</td></tr><tr><td>Version</td><td></td><td></td><td></td><td></td></tr><tr><td>General Capabilities</td><td></td><td></td><td></td><td></td></tr><tr><td>WQ Capabilities</td><td></td><td></td><td></td><td></td></tr><tr><td>Group Capabilities</td><td></td><td></td><td></td><td></td></tr><tr><td>Engine Capabilities</td><td colspan="4">Contain read-only values indicating capabilities of the device.</td></tr><tr><td>Operations Capabilities</td><td></td><td></td><td></td><td></td></tr><tr><td>Command Capabilities</td><td></td><td></td><td></td><td></td></tr><tr><td>Perfmon Capabilities</td><td></td><td></td><td></td><td></td></tr><tr><td>Inter-Domain Capabilities</td><td></td><td></td><td></td><td></td></tr><tr><td>Table Offsets</td><td></td><td></td><td></td><td></td></tr><tr><td>General Configuration1</td><td colspan="4">Global Read Buffer Limit: initialized to Total Read BuffersAll other fields: 0</td></tr><tr><td>General Control</td><td></td><td></td><td></td><td></td></tr><tr><td>General Status</td><td>0</td><td>0</td><td>0</td><td>0</td></tr><tr><td>Interrupt Cause</td><td></td><td></td><td></td><td></td></tr><tr><td>MSI-X Pending Bit Array</td><td></td><td></td><td></td><td></td></tr><tr><td>MSI-X Permissions Table</td><td></td><td></td><td></td><td></td></tr><tr><td>Event Log Configuration</td><td></td><td></td><td></td><td></td></tr><tr><td>Command Parameter</td><td></td><td></td><td></td><td></td></tr><tr><td>Inter-Domain Permissions</td><td></td><td></td><td></td><td></td></tr><tr><td>Table</td><td></td><td></td><td></td><td></td></tr><tr><td>Inter-Domain Bitmap</td><td></td><td></td><td></td><td></td></tr><tr><td>WQ Configuration1</td><td colspan="4">OPCFG: Initialized to match the value in OPCAPAll other fields: 0</td></tr><tr><td>Group Configuration1</td><td colspan="4">Read Buffers Allowed: initialized to Total Read BuffersAll other fields: 0</td></tr><tr><td>Command</td><td></td><td></td><td></td><td></td></tr><tr><td>Command Status</td><td>0</td><td>0</td><td>0</td><td>Preserved</td></tr><tr><td>Software Error</td><td></td><td></td><td></td><td></td></tr><tr><td>Event Log Status</td><td></td><td></td><td></td><td></td></tr><tr><td>Perfmon</td><td colspan="4">PASID Filter Configuration Registers: Initialized to 0x3FFFFFF.Other Filter Configuration Registers: Initialized to 0xFFFF.For all other registers: 0</td></tr><tr><td>MSI-X Table</td><td colspan="3">Message Data: 0Message Address: 0Mask: 1</td><td>Preserved</td></tr></table>

<table><tr><td>Interrupt Message Storage</td><td>Message Data: 0
Message Address: 00000000FEE00000
Mask: 1
PASID, PASID Enable, Ignore, Pending: 0</td></tr></table>


Table 9-3: MMIO Register Initial Values



The following MMIO-space registers are read-only under the described conditions:


<table><tr><td>Register</td><td>Conditions Under Which Register is Read-Only</td></tr><tr><td>General Configuration Group Configuration</td><td>While device is not Disabled.</td></tr><tr><td>WQ Configuration</td><td>See Table 9-7.</td></tr><tr><td>Perfmon</td><td>See Table 9-8.</td></tr><tr><td>Inter-Domain Permissions Table</td><td>See Table 9-10.</td></tr><tr><td>Event Log Configuration</td><td>While device is not Disabled.</td></tr></table>

Table 9-4: Read-Only MMIO Registers

# 9.2.1 Version Register (VERSION)

The Version register reports the version of this architecture specification that is supported by the device.

<table><tr><td colspan="4">VERSION
Base: BAR0
Offset: 0x0
Size: 4 bytes (32 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:16</td><td>RO</td><td>16 bits</td><td>Unused.</td></tr><tr><td>15:8</td><td>RO</td><td>8 bits</td><td>Major version</td></tr><tr><td>7:0</td><td>RO</td><td>8 bits</td><td>Minor version</td></tr></table>

# 9.2.2 General Capabilities Register (GENCAP)

<table><tr><td colspan="4">GENCAP
Base: BARO
Offset: 0x10
Size: 8 bytes (64 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:55</td><td>RO</td><td>9 bits</td><td>Unused</td></tr><tr><td>54</td><td>RO</td><td>1 bit</td><td>Strict Ordering Limitation for Peer Destinations
0: Strict ordering is supported for descriptors whose destination address references a peer device.
1: The Strict Ordering flag is reserved in descriptors whose destination address references a peer device.</td></tr><tr><td>53</td><td>RO</td><td>1 bit</td><td>Strict Ordering Limitation for Memory Destinations
0: Strict ordering is supported for descriptors whose destination address references memory.
1: The Strict Ordering flag is reserved in descriptors whose destination address references memory.</td></tr><tr><td>52</td><td>RO</td><td>1 bit</td><td>Batch1 Support
0: For a batch operation, Descriptor count must be greater than 1.
1: For a batch operation, Descriptor count must be greater than or equal to 1. The WQ Maximum Batch Size field in WQCFG is allowed to be 0.</td></tr><tr><td>51:33</td><td>RO</td><td>19 bits</td><td>Unused</td></tr><tr><td>32</td><td>RO</td><td>1 bit</td><td>Event Log Overflow Support
0: Event Log does not overflow. If the Event Log is full at the time hardware attempts to append an entry, the device blocks until the Event Log Head field is updated by software.
1: Event Log may overflow. If the Event Log is full at the time hardware attempts to append an entry, hardware drops the event, and attempts to log an error in SWERROR to indicate the Event Log full condition. If the Valid bit in SWERROR is 1 and the Event Log is full at the time an event occurs, the Overflow bit is SWERROR is set to 1.</td></tr><tr><td>31</td><td>RO</td><td>1 bit</td><td>Configuration Support
0: General Configuration, Group Configuration, and some fields of the Work Queue Configuration registers are read-only and reflect the fixed configuration of the device. See section 9.2.24 for details about which WQ configuration fields are read-only.
1: General Configuration, Group Configuration, and Work Queue Configuration registers are read-write and can be used by software to set the desired configuration.</td></tr><tr><td>30:25</td><td>RO</td><td>6 bits</td><td>Interrupt Message Storage Size
The number of entries in the Interrupt Message Storage is N × 256, where N is the value in this field.</td></tr><tr><td>24:21</td><td>RO</td><td>4 bits</td><td>Maximum Supported Batch Size
The maximum number of descriptors that can be referenced by a Batch descriptor is independently controlled for each WQ. This field indicates the maximum value that each WQ can be configured with. The maximum supported batch size is 2N, where N is the value in this field. If Batch descriptor is not supported (as indicated by bit 1 of OPCAP), this field is unused.</td></tr><tr><td>20:16</td><td>RO</td><td>5 bits</td><td>Maximum Supported Transfer Size
The maximum transfer size that can be specified in a descriptor is inde-pendently controlled for each WQ. This field indicates the maximum value that each WQ can be configured with. The maximum supported transfer size is 2N, where N is the value in this field.</td></tr><tr><td>15</td><td>RO</td><td>1 bit</td><td>Batch Continuation Support
Indicates support for the Batch Error flag and the Result field in the Batch completion record.</td></tr><tr><td>14:13</td><td>RO</td><td>2 bits</td><td>Event Log Support
0: No Event Log Support.
1: 64-byte entries.
2: 128-byte entries.
3: 256-byte entries.
See section 5.9 for more information on Event Log.</td></tr><tr><td>12</td><td>RO</td><td>1 bit</td><td>Completion Record Fault Info Support
0: Completion Record does not have a Fault Info field.
1: Completion Record has a Fault Info field, as described in section 8.2.3.</td></tr><tr><td>11</td><td>RO</td><td>1 bit</td><td>CRC64 Support
0: The CRC Size flag in a CRC Generation or Copy with CRC Generation descriptor is not supported.
1: The CRC Size flag in a CRC Generation or Copy with CRC Generation descriptor is supported.</td></tr><tr><td>10</td><td>RO</td><td>1 bit</td><td>FillI6 Support
0: The Pattern Size flag in a Fill or Inter-Domain Memory Fill descriptor is not supported.
1: The Pattern Size flag in a Fill or Inter-Domain Memory Fill descriptor is supported.</td></tr><tr><td>9</td><td>RO</td><td>1 bit</td><td>Drain Descriptor Readback Address Support
0: Hardware does not support specification of Readback Addresses in Drain descriptors and the Readback Address Valid flags and the Readback Address fields in the descriptor are reserved.
1: Hardware supports specification of Readback Addresses in Drain descriptors. If the corresponding Readback Address Valid flags are set, hardware will issue the corresponding readbacks.</td></tr><tr><td>8</td><td>RO</td><td>1 bit</td><td>Destination Readback Support
0: Cache control hints in section 8.1.3.1 that specify destination readback are not supported.
1: Cache control hints that specify destination readback are supported.</td></tr><tr><td>7</td><td>RO</td><td>1 bit</td><td>Translation Fetch Stride Support
0: The Use Stride flag in a Translation Fetch descriptor is not supported. Hardware uses an implementation specific stride value.
1: The Use Stride flag in a Translation Fetch descriptor is supported.</td></tr><tr><td colspan="4">GENCAP
Base: BAR0
Offset: 0x10
Size: 8 bytes (64 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>6</td><td>RO</td><td>1 bit</td><td>Inter-Domain Support
0: Inter-Domain operations are not supported.
1: Inter-Domain operations are supported while the PASID capability is enabled. Inter-Domain capabilities are reported in the IDCAP register (section 9.2.18).</td></tr><tr><td>5</td><td>RO</td><td>1 bit</td><td>Durable Write Support
0: Cache control hints in section 8.1.3.1 that specify writes to durable memory are not supported, unless stated otherwise in Table 8-4.
1: Cache control hints that specify writes to durable memory are supported.</td></tr><tr><td>4</td><td>RO</td><td>1 bit</td><td>Command Capabilities Support
0: The Command Capabilities register is not supported. The administrative commands supported are listed in Table 9-6.
1: The Command Capabilities register is supported and reports the set of administrative commands supported by the device. See section 9.2.14 for a description of the Command Capabilities register.</td></tr><tr><td>3</td><td>RO</td><td>1 bit</td><td>Unused.</td></tr><tr><td>2</td><td>RO</td><td>1 bit</td><td>Cache Control Support
0: Cache control hints in section 8.1.3.1 that specify writing data to the cache hierarchy are not supported.
1: Cache control hints that specify writing data to cache are supported.</td></tr><tr><td>1</td><td>RO</td><td>1 bit</td><td>Overlapping Copy Support
0: Overlapping copies are not supported. If source and destination buffers overlap, it is an error.
1: Overlapping copies are supported by the Memory Move operation. See the description of the Memory Move operation for details of the behavior.
Regardless of the value of this field, overlapping copies are not supported by any operation other than Memory Move.</td></tr><tr><td>0</td><td>RO</td><td>1 bit</td><td>Block on Fault Support
0: Block on fault is not supported. The Block On Fault Enable bit in the WQCFG registers and the Block On Fault flag in descriptors are reserved. If a page fault occurs on a source or destination memory access, the operation stops and the page fault is reported to software.
1: Block on fault is supported. Behavior on page faults depends on the values of the Block On Fault Enable bit in each WQCFG register and the Block on Fault flag in each descriptor.
See section 3.13 for more information on page fault handling.</td></tr></table>

# 9.2.3 WQ Capabilities Register (WQCAP)

<table><tr><td colspan="4">WQCAP
Base: BARO
Offset: 0x20
Size: 8 bytes (64 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:56</td><td>RO</td><td>8 bits</td><td>Unused.</td></tr><tr><td>55</td><td>RO</td><td>1 bit</td><td>WQ PRS Support
0: The WQ PRS Disable control is not supported.
1: The WQ PRS Disable control may be used to control the use of PRS independently for each WQ.</td></tr><tr><td>54</td><td>RO</td><td>1 bit</td><td>WQ Operations Configuration Support
0: Configuration of per WQ operations is not supported. The OPCFG fields in WQCFG are reserved. All WQs support the operations specified in the OPCAP register.
1: Configuration of per WQ operations is supported. The OPCFG fields in WQCFG may be used to limit the operations allowed in each WQ.</td></tr><tr><td>53</td><td>RO</td><td>1 bit</td><td>WQ Occupancy Interrupt Support
0: WQ occupancy interrupts are not supported. The WQ Occupancy Limit and WQ Occupancy Interrupt Enable fields in WQCFG are reserved.
1: WQ occupancy interrupts are supported as described in section 9.2.24.</td></tr><tr><td>52</td><td>RO</td><td>1 bit</td><td>WQ Occupancy Support
0: The value of the WQ Occupancy field in WQCFG is undefined.
1: The WQ Occupancy field in WQCFG contains the current occupancy of the WQ.</td></tr><tr><td>51</td><td>RO</td><td>1 bit</td><td>WQ Priority Support
0: WQ priorities are not supported. The WQ Priority field in WQ configuration is ignored.
1: WQ priorities are supported as described in section 4.1.</td></tr><tr><td>50</td><td>RO</td><td>1 bit</td><td>WQ ATS Support
0: ATS is used for all WQs according to the setting of the Enable field in the PCIe ATS capability.
1: The WQ ATS Disable control may be used to control the use of ATS independently for each WQ.</td></tr><tr><td>49</td><td>RO</td><td>1 bit</td><td>Dedicated Mode Support
0: Dedicated mode is not supported. All WQs must be configured in shared mode.
1: Dedicated mode is supported.</td></tr><tr><td>48</td><td>RO</td><td>1 bit</td><td>Shared Mode Support
0: Shared mode is not supported. All WQs must be configured in dedicated mode.
1: Shared mode is supported.</td></tr><tr><td>47:28</td><td>RO</td><td>20 bits</td><td>Unused.</td></tr><tr><td>27:24</td><td>RO</td><td>4 bits</td><td>WQCFG Size
Indicates the size of the WQCFG register for each WQ. The size of each WQCFG register is 2N+5 bytes, where N is the value in this field.</td></tr><tr><td>23:16</td><td>RO</td><td>8 bits</td><td>Number of WQs</td></tr><tr><td colspan="4">WQCAP
Base: BAR0
Offset: 0x20
Size: 8 bytes (64 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>15:0</td><td>RO</td><td>16 bits</td><td>Total WQ Size
The total amount of work queue space in the device, which may vary in different implementations. Software uses the WQCFG registers to apportion this space among the WQs, to support multiple QoS levels and/or multiple dedicated work queues.</td></tr></table>

# 9.2.4 Group Capabilities Register (GRPCAP)

<table><tr><td colspan="4">GRPCAP
Base: BARO
Offset: 0x30
Size: 8 bytes (64 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:20</td><td>RO</td><td>44 bits</td><td>Unused.</td></tr><tr><td>19</td><td>RO</td><td>1 bit</td><td>Bandwidth Limit Support
0: The Read Bandwidth Limit and Write Bandwidth Limit fields in GRPFLAGS are reserved.
1: The Read Bandwidth Limit and Write Bandwidth Limit fields in GRPFLAGS can be used to limit the maximum read and write bandwidth for each engine in the group.</td></tr><tr><td>18</td><td>RO</td><td>1 bit</td><td>Descriptors in Progress Limit Supported
0: The Work Descriptors in Progress Limit and Batch Descriptors in Progress Limit fields in GRPFLAGS are reserved.
1: The Work Descriptors in Progress Limit and Batch Descriptors in Progress Limit fields in GRPFLAGS can be used by software to control the value for each engine in the group.</td></tr><tr><td>17</td><td>RO</td><td>1 bit</td><td>Global Read Buffer Limit Supported
0: The Global Read Buffer Limit field of GENCFG and the Use Global Read Buffer Limit field in GRPCFG are reserved.
1: Global Read Buffer Limit and Use Global Read Buffer Limit can be used by software to control bandwidth usage by selected groups, as described in chapter 4.</td></tr><tr><td>16</td><td>RO</td><td>1 bit</td><td>Read Buffer Controls Supported
0: The Read Buffers Allowed and Read Buffers Reserved fields in GRPCFG are read-only and are unused. The value in the Total Read Buffers field is undefined.
1: Read Buffers Allowed and Read Buffers Reserved are supported as described in chapter 4.</td></tr><tr><td>15:8</td><td>RO</td><td>8 bits</td><td>Total Read Buffers
Indicates the total number of Read Buffers available. See chapter 4 for information on the meaning of this field.</td></tr><tr><td>7:0</td><td>RO</td><td>8 bits</td><td>Number of Groups</td></tr></table>

# 9.2.5 Engine Capabilities Register (ENGCAP)

<table><tr><td colspan="4">ENGCAP
Base: BARO
Offset: 0x38
Size: 8 bytes (64 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:24</td><td>RO</td><td>40 bits</td><td>Unused.</td></tr><tr><td>23:16</td><td>RO</td><td>8 bits</td><td>Maximum Batch Descriptors in Progress
The maximum number of batch descriptors that each engine is capable of concurrently processing at any time. This field is undefined if the Descriptors in Progress Limit Supported field is GRPCAP is 0.</td></tr><tr><td>15:8</td><td>RO</td><td>8 bits</td><td>Maximum Work Descriptors in Progress
The maximum number of work descriptors that each engine is capable of concurrently processing at any time. This field is undefined if the Descriptors in Progress Limit Supported field is GRPCAP is 0.</td></tr><tr><td>7:0</td><td>RO</td><td>8 bits</td><td>Number of Engines</td></tr></table>

# 9.2.6 Operations Capabilities Register (OPCAP)

The Operations Capabilities register indicates which operation types are supported by the device. The register is a bitmask where each bit corresponds to the operation type with the same code as the bit position. For example, bit 0 of this register corresponds to the No-op operation (code 0). See section 8.1 for the values of the operation codes.

<table><tr><td colspan="3">OPCAP
Base: BAR0</td><td>Offset: 0x40</td><td>Size: 32 bytes (4 × 64 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td colspan="2">Description</td></tr><tr><td>255:0</td><td>RO</td><td>256 bits</td><td colspan="2">Each bit corresponds to an operation code and indicates whether that operation type is supported. If the bit is 1, the corresponding operation type is supported; if the bit is 0, the corresponding operation type is not supported. Bits corresponding to undefined operation codes are unused and are read as 0.</td></tr></table>

# 9.2.7 Table Offsets Register (OFFSETS)

Hardware implementations may place configuration tables in any otherwise unassigned address ranges within BARO MMIO space. This register indicates the offsets of these tables: Group Configuration, WQ Configuration, MSI-X Permissions, IMS, Performance Monitoring, and IDPT. Software must use the values in this register to determine the offsets of these tables, as the offsets may change between implementations.

<table><tr><td colspan="4">OFFSETSSe</td></tr><tr><td colspan="3">Base: BARO</td><td>Offset: 0x60 Size: 16 bytes (2 x 64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>127:96</td><td>RO</td><td>32 bits</td><td>Unused.</td></tr><tr><td>95:80</td><td>RO</td><td>16 bits</td><td>Inter-Domain Permissions Table OffsetIndicates the offset of the Inter-Domain Permissions Table. The offset is the value in this field times 0x100.</td></tr><tr><td>79:64</td><td>RO</td><td>16 bits</td><td>Perfmon OffsetIndicates the offset of the Performance Monitoring Registers. The offset is the value in this field times 0x100.</td></tr><tr><td>63:48</td><td>RO</td><td>16 bits</td><td>IMS OffsetIndicates the offset of the Interrupt Message Storage. The offset is the value in this field times 0x100.</td></tr><tr><td>47:32</td><td>RO</td><td>16 bits</td><td>MSI-X Permissions OffsetIndicates the offset of the MSI-X Permissions Table. The offset is the value in this field times 0x100.</td></tr><tr><td>31:16</td><td>RO</td><td>16 bits</td><td>WQ Configuration OffsetIndicates the offset of the WQ Configuration Table. The offset is the value in this field times 0x100.</td></tr><tr><td>15:0</td><td>RO</td><td>16 bits</td><td>Group Configuration OffsetIndicates the offset of the Group Configuration Table. The offset is the value in this field times 0x100.</td></tr></table>

# 9.2.8 General Configuration Register (GENCFG)

This register is read-write while the device is Disabled and read-only otherwise. Some fields are read-only at all times if the Configuration Support field in GENCAP is 0.

<table><tr><td colspan="4">GENCFG
Base: BARO
Offset: 0x80
Size: 4 bytes (32 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:14</td><td>RSVD</td><td>18 bits</td><td>Reserved.</td></tr><tr><td>13</td><td>RWL</td><td>1 bit</td><td>Event Log Enable
0: Hardware writes any software error related events to SWERROR.
1: Any events including software error related ones are written to the event log in memory.
See section 5.9 for information on the event log.
This field is reserved when Event Log Support in GENCAP is 0.</td></tr><tr><td>12</td><td>RWL</td><td>1 bit</td><td>User-mode Interrupts Enable
0: User-mode descriptors are not allowed to request completion interrupts.
1: User-mode descriptors may request completion interrupts. An application is prevented from generating any interrupt that is not assigned to it by matching the PASID field of the interrupt table entry to the PASID of the descriptor. See section 5.4.
This field is read-only if the Configuration Support field in GENCAP is 0.</td></tr><tr><td>11:8</td><td>RSVD</td><td>4 bits</td><td>Reserved.</td></tr><tr><td>7:0</td><td>RWL</td><td>8 bits</td><td>Global Read Buffer Limit
This field indicates the maximum number of Read Buffers that may be in use at one time by operations that access low bandwidth memory. This number of Read Buffers is shared by all descriptors accessing low bandwidth memory across the entire device. The default value is equal to the Total Read Buffers reported in GRPCAP.
The value in this field is used when the Use Global Read Buffer Limit field in any of the Group Configuration registers is 1. See section 9.2.23. If used, this value must be at least 4 times the total number of engines in all groups that have the Use Global Read Buffer Limit set to 1.
If the Global Read Buffer Limit Supported field in GRPCAP is 0, this field is reserved.
This field is read-only if the Configuration Support field in GENCAP is 0.</td></tr></table>

# 9.2.9 General Control Register (GENCTRL)

<table><tr><td colspan="4">GENCTRL
Base: BAR0
Offset: 0x88
Size: 4 bytes (32 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:3</td><td>RSVD</td><td>29 bits</td><td>Reserved.</td></tr><tr><td>2</td><td>RW</td><td>1 bit</td><td>Event Log Interrupt Enable
0: No interrupt is generated when an event is written to the event log.
1: The interrupt at index 0 in the MSI-X table is generated when an event is written to the event log. The Event Log field of the Interrupt Cause Register is set to 1.</td></tr><tr><td>1</td><td>RW</td><td>1 bit</td><td>Halt State Interrupt Enable
0: No interrupt is generated when device transitions to Halt state.
1: The interrupt at index 0 in the MSI-X table is generated when the device transitions to Halt state (see section 5.6). The Halt State field of the Interrupt Cause Register is set to 1.</td></tr><tr><td>0</td><td>RW</td><td>1 bit</td><td>Software Error Interrupt Enable
0: No interrupt is generated for software errors.
1: The interrupt at index 0 in the MSI-X table is generated when the Valid field in SWERROR changes from 0 to 1. The Software Error field of the Interrupt Cause Register is set to 1.</td></tr></table>

# 9.2.10 General Status Register (GENSTS)

<table><tr><td colspan="4">GENSTS
Base: BARO
Offset: 0x90
Size: 4 bytes (32 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:4</td><td>RO</td><td>28 bits</td><td>Unused.</td></tr><tr><td>3:2</td><td>RO</td><td>2 bits</td><td>Reset Type Required
00: Software can issue a Reset Device command to the Command Register (see section 9.2.12) to recover the device.
01: Device requires a function-level reset (FLR) to recover from the current state.
10: Device requires a warm-reset to recover from the current state.
11: Device requires a cold-reset to recover. This is typically after a severe error that cannot be cleared with a function-reset (FLR) or warm reset. This field indicates the minimum reset type needed to recover. Software can choose to invoke a stronger type of reset to reinitialize the device.
The mechanism used to trigger a warm reset or cold reset may be platform-specific.
When using Function Level Reset, software is expected to follow the app note in the PCIe specification, section 6.6.2.</td></tr><tr><td>1:0</td><td>RO</td><td>2 bits</td><td>Device State
00: Device is Disabled. No work is performed. All ENQ operations return Retry.
01: Device is Enabled. Work queues may be enabled, and descriptors may be submitted to enabled work queues.
10: Disable Device or Reset Device command is in progress. Descriptors are not accepted into any WQ. All Descriptors are being drained.
11: Halt State. The device is halted due to an error or unsupported condition that was encountered. Additional details related to this state and related software actions needed are described in section 5.7.</td></tr></table>

# 9.2.11 Interrupt Cause Register (INTCAUSE)

The Interrupt Cause Register is used to indicate the reason that an interrupt was generated using entry 0 in the MSI-X table. For interrupts generated using other MSI-X table entries or any of the IMS entries, no separate cause register exists. In the latter cases, software can identify the cause of the interrupt based on the interrupt vector or by reading the cause associated location, for example the completion record address or the WQ Occupancy register.

<table><tr><td colspan="4">INTCAUSE
Base: BARO
Offset: 0x98
Size: 4 bytes (32 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31</td><td>RW1C</td><td>1 bit</td><td>Interrupt Handles Revoked</td></tr><tr><td>30:6</td><td>RSVD</td><td>25 bits</td><td>Reserved</td></tr><tr><td>5</td><td>RW1C</td><td>1 bit</td><td>Event Log</td></tr><tr><td>4</td><td>RW1C</td><td>1 bit</td><td>Halt State</td></tr><tr><td>3</td><td>RW1C</td><td>1 bit</td><td>Perfmon Counter Overflow</td></tr><tr><td>2</td><td>RW1C</td><td>1 bit</td><td>WQ Occupancy Below Limit</td></tr><tr><td>1</td><td>RW1C</td><td>1 bit</td><td>Command Completion</td></tr><tr><td>0</td><td>RW1C</td><td>1 bit</td><td>Software Error</td></tr></table>

# 9.2.12 Command Register(CMD)

The Command register is used to submit administrative commands. Before writing to this register, software must ensure that any command previously submitted via this register has completed by checking the Active field of the Command Status register. When a command is submitted, the Active field of the Command Status register is set to 1. The Active field changes to 0 when the command is complete. The other fields of the Command Status register indicate whether the command completed successfully. If the command register is written while Active is 1, the value written is discarded and an error is recorded in the SWERROR register. Reading the Command register returns unpredictable values.

When the command finishes, if the Request Completion Interrupt field of the Command register is 1, then the Command Completion field of the Interrupt Cause register is set to 1 and an interrupt is generated using entry 0 in the MSI-X table.

The Command Capabilities register (9.2.14) indicates which of the commands listed in Table 9-5 are supported by an implementation. If an undefined or unsupported command is written to the Command register, error code 0x01 is reported in the Command Status register.

Some implementations may not check reserved fields in the Command register, but software should take care to write 0 to all unused fields for maximum compatibility.

See section 3.13.3 for details on the operation of commands submitted to the Command register.

<table><tr><td colspan="4">CMD
Base: BAR0
Offset: 0xA0
Size: 4 bytes (32 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31</td><td>WO</td><td>1 bit</td><td>Request Completion Interrupt
When this field is 1, upon completion of the command an interrupt is generated using entry 0 in the MSI-X table.</td></tr><tr><td>30:25</td><td>RSVD</td><td>6 bits</td><td>Reserved.</td></tr><tr><td>24:20</td><td>WO</td><td>5 bits</td><td>Command Code
See Table 9-5 for command codes. Undefined command codes are reserved.</td></tr><tr><td>19:0</td><td>WO</td><td>20 bits</td><td>Operand
The meaning of this field depends on the command. See Table 9-5.</td></tr></table>

<table><tr><td>Command</td><td>Code</td><td colspan="2">Operand</td><td>Operation</td></tr><tr><td>Enable Device</td><td>1</td><td colspan="2">Reserved</td><td>Enable the device.</td></tr><tr><td>Disable Device</td><td>2</td><td colspan="2">Reserved</td><td>Disable the device.</td></tr><tr><td>Drain All</td><td>3</td><td colspan="2">Reserved</td><td>Wait for all descriptors.</td></tr><tr><td>Abort All</td><td>4</td><td colspan="2">Reserved</td><td>Abandon and/or wait for all descriptors.</td></tr><tr><td>Reset Device</td><td>5</td><td colspan="2">Reserved</td><td>Disable the device and clear the device configuration.</td></tr><tr><td>Command</td><td>Code</td><td colspan="2">Operand</td><td>Operation</td></tr><tr><td>Enable WQ</td><td>6</td><td colspan="2">19:8 7:0 Reserved WQ to enable</td><td>Enable the WQ.</td></tr><tr><td>Disable WQ</td><td>7</td><td colspan="2" rowspan="4">19:16 Group number1 15:0 Bitmap specify-ing which WQs in the group to operate on. See description below.</td><td>Disable the specified WQs.</td></tr><tr><td>Drain WQ</td><td>8</td><td>Wait for descriptors in the specified WQs.</td></tr><tr><td>Abort WQ</td><td>9</td><td>Abandon and/or wait for descriptors in the specified WQs.</td></tr><tr><td>Reset WQ</td><td>10</td><td>Disable the specified WQs and clear the WQ configurations.</td></tr><tr><td>Drain PASID</td><td>11</td><td colspan="2">The PASID to drain.</td><td>Wait for descriptors using the specified PASID.</td></tr><tr><td>Abort PASID</td><td>12</td><td colspan="2">The PASID to abort.</td><td>Abandon and/or wait for descriptors using the specified PASID.</td></tr><tr><td>Request Interrupt Handle</td><td>13</td><td colspan="2">19:17 Reserved 16 0: MSI-X table 1: IMS 15:0 Table index</td><td>Return a handle for the specified interrupt table entry. If this command is supported, it must be used to obtain interrupt handles. See section 3.7 for more information.</td></tr><tr><td>Release Interrupt Handle</td><td>14</td><td colspan="2">19:17 Reserved 16 0: MSI-X table 1: IMS 15:0 Table index</td><td>Release the handle that was returned by Request Interrupt Handle for the specified interrupt table entry.</td></tr><tr><td>Request IDPT Handle</td><td>15</td><td colspan="2">19:16 Reserved 15:0 IDPT Index</td><td>Request a handle for the specified index in the Inter-Domain Permissions Table. - If the Request IDPT Handle capability in CMDCAP is 1, software is required to use this command to obtain an IDPT handle. The returned handle may be used in an inter-domain or Update Window descriptor. - If the Request IDPT Handle capability in CMDCAP is 0, this command code is reserved. The IDPT index may be used as the handle in an inter-domain or Update Window descriptor.</td></tr><tr><td>Release IDPT Handle</td><td>16</td><td colspan="2">19:16 Reserved 15:0 IDPT Index</td><td>Release the IDPT handle that was returned by Request IDPT Handle for the specified IDPT entry. - If the Release IDPT Handle capability in CMDCAP is 1, software may use this</td></tr><tr><td>Command</td><td>Code</td><td colspan="2">Operand</td><td>Operation</td></tr><tr><td></td><td></td><td></td><td colspan="2">command to release an IDPT entry that is no longer in use.
- If the Release IDPT Handle capability in CMDCAP is 0, this command code is reserved.</td></tr><tr><td>Invalidate Submitter Bitmap Cache</td><td>17</td><td>CMD
19:17 Reserved
16:0 Size
CMDPARAM
63:0 Address</td><td colspan="2">Invalidate the specified address range if cached in the submitter bitmap cache. The Address and Size fields specify the portion of the bitmap to be invalidated.
The Address parameter must be written to the CMDCARAM register prior to issuing this command to the Command register.
- If the Invalidate Submitter Bitmap Cache field in CMDCAP is 1, hardware may cache any portion of a bitmap, and software is required to issue this command after any change to a bitmap region including page mapping changes, and after performing necessary invalidations for any pages that are part of a bitmap.
- If the Invalidate Submitter Bitmap Cache field in CMDCAP is 0, this command code is reserved, and software is not required to issue this command after any change to a bitmap.
See section 3.14.2 for details related to bitmap caching.</td></tr></table>

Table 9-5: Administrative Commands

The Disable WQ, Drain WQ, Abort WQ, and Reset WQ commands can be applied to groups<sup>1</sup> of up to 16 WQs at the same time. The group number is specified in bits 19:16 of the operand field, corresponding to bits 7:4 of the WQ index. Bits 15:0 of the operand field contain a bitmask indicating which WQs in the group to operate on. In an implementation with no more than 16 WQs, the group number is always 0. For example, to drain WQs 1, 4, and 7, theOperand field would be set to 0x00092. To drain WQs 21 and 22, theOperand field would be set to 0x10060. It is not possible use a single command to disable or drain WQs in different groups.

# 9.2.13 Command Status Register (CMDSTATUS)

The Command Status register indicates the status of the last command submitted to the Command register. The Active field indicates that a command is in progress. The Active field is set to 1 when a command is written to the Command register. While the Active field is 1, the values of the other fields are unspecified. When the command completes, the Active field is set to 0 and the other fields of this register indicate whether the command completed successfully.

<table><tr><td colspan="4">CMDSTATUS
Base: BARO
Offset: 0xA8
Size: 4 bytes (32 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31</td><td>RO</td><td>1 bit</td><td>Active
0: Command is complete (or no command has been submitted).
1: Command is in progress.</td></tr><tr><td>30:24</td><td>RSVD</td><td>7 bits</td><td>Unused.</td></tr><tr><td>23:8</td><td>RO</td><td>16 bits</td><td>Command Result
For the Request Interrupt Handle command, if the Error Code field is 0, this field contains the interrupt handle corresponding to the interrupt table entry specified in the command operand. If Error Code is non-zero, this field is unused.
For the Request IDPT Handle command, if the Error Code field is 0, this field contains the IDPT handle corresponding to the IDPT entry speci-fied in the command operand. If Error Code is non-zero, this field is unused.
For any other command, this field is unused.</td></tr><tr><td>7:0</td><td>RO</td><td>8 bits</td><td>Error Code
0x00: Successful completion.
0x01: Undefined or unsupported command code.
0x02: Invalid WQ index.
0x03: Error Condition caused by a platform or internal hardware error.
Software can read GENSTS register and PCIe AER logs for details and to determine further action.
0x04: Non-zero reserved field in command.
0x05: Command submitted while device is in halt or error state.
0x06-0x0f: Unused.
0x10-0xff: Command-specific error codes. See Table 5-8.</td></tr></table>

# 9.2.14 Command Capabilities Register (CMDCAP)

The Command Capabilities register indicates which administrative commands are supported by the Command register. This register is a bitmask where each bit corresponds to the command with the same command code as the bit position. For example, bit 1 of this register corresponds to the Enable Device command (command code 1). See Table 9-5 for the values of the command codes.

This register is present only if the Command Capabilities Support field in GENCAP is 1.

If this register indicates support for the Request Interrupt Handle command, then the command must be used to obtain interrupt handles to use for descriptor completions.

<table><tr><td colspan="4">CMDCAP
Base: BAR0
Offset: 0xB0
Size: 8 bytes (64 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:0</td><td>RO</td><td>64 bits</td><td>Each bit corresponds to a command code, and indicates whether that administrative command is supported. If the bit is 1, the corresponding command is supported; if the bit is 0, the corresponding command is not supported. Bits corresponding to undefined command codes are unused and are read as 0.</td></tr></table>

If Command Capabilities Support is 0, this register is not present and the following commands are supported:

<table><tr><td>Command</td><td>Code</td><td>Operation</td></tr><tr><td>Enable Device</td><td>1</td><td>Enable the device.</td></tr><tr><td>Disable Device</td><td>2</td><td>Disable the device.</td></tr><tr><td>Drain All</td><td>3</td><td>Wait for all descriptors.</td></tr><tr><td>Abort All</td><td>4</td><td>Abandon and/or wait for all descriptors.</td></tr><tr><td>Reset Device</td><td>5</td><td>Disable the device and clear the device configuration.</td></tr><tr><td>Enable WQ</td><td>6</td><td>Enable the WQ.</td></tr><tr><td>Disable WQ</td><td>7</td><td>Disable the specified WQs.</td></tr><tr><td>Drain WQ</td><td>8</td><td>Wait for descriptors in the specified WQs.</td></tr><tr><td>Abort WQ</td><td>9</td><td>Abandon and/or wait for descriptors in the specified WQs.</td></tr><tr><td>Reset WQ</td><td>10</td><td>Disable the specified WQs and clear the WQ configurations.</td></tr><tr><td>Drain PASID</td><td>11</td><td>Wait for descriptors using the specified PASID.</td></tr><tr><td>Abort PASID</td><td>12</td><td>Abandon and/or wait for descriptors using the specified PASID.</td></tr></table>

Table 9-6: Default Commands Supported

# 9.2.15 Software Error Register (SWERROR)

Several types of errors can be recorded in this register:

An error in submitting a descriptor.

- An error translating a Completion Record Address in a descriptor.

An error validating a descriptor, if the Completion Record Address Valid flag in the descriptor is 0

- An error while processing a descriptor, such as a page fault, if the Completion Record Address Valid flag in the descriptor is 0.

An unsupported change to device configuration while the device is not Disabled.

Details on the error checking that can result in these errors are covered in chapter 5.

Only one error at a time can be recorded in this register. When an error is recorded, Valid is set to 1. If Valid is 1 at the time an error occurs, Overflow is set to 1 and the error is not recorded. The Valid and Overflow fields are cleared by software writing 1. They are not cleared by hardware, other than by reset. When supported, the event log may be used for reporting multiple errors without overflow (as described in section 5.9).

When Valid changes from 0 to 1, if the Software Error Interrupt Enable field in GENCTRL is 1, the Software Error field of the Interrupt Cause register is set to 1 and an interrupt is generated.

<table><tr><td colspan="5">SWERROR
Base: BARO
Offset: 0xC0
Size: 32 bytes (4 × 64 bits)</td></tr><tr><td>Byte offset</td><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td rowspan="8">7:0</td><td>63:60</td><td>RO</td><td>4 bits</td><td>Unused.</td></tr><tr><td>59:40</td><td>RO</td><td>20 bits</td><td>PASID
The PASID field of the descriptor that caused the error.</td></tr><tr><td>39:32</td><td>RO</td><td>8 bits</td><td>Operation
The Operation field of the descriptor that caused the error.</td></tr><tr><td>31:24</td><td>RO</td><td>8 bits</td><td>Unused.</td></tr><tr><td>23:16</td><td>RO</td><td>8 bits</td><td>WQ Index
Indicates which WQ the descriptor was submitted to.</td></tr><tr><td>15:8</td><td>RO</td><td>8 bits</td><td>Error code
See section 5.8 for the meaning of the value in this field.</td></tr><tr><td>7</td><td>RO</td><td>1 bit</td><td>Error Information Valid
0: Error Information field is valid only if the Error code is Invalid
Flags (0x11).
1: Error Information field is valid and provides additional information pertaining to the error reported in the Error code field.</td></tr><tr><td>6</td><td>RO</td><td>1 bit</td><td>Priv
The Priv field of the descriptor that caused the error.</td></tr></table>

<table><tr><td colspan="6">SWERROR
Base: BARO
Offset: 0xCO
Size: 32 bytes (4 × 64 bits)</td></tr><tr><td>Byte offset</td><td>Bits</td><td>Attr</td><td>Size</td><td colspan="2">Description</td></tr><tr><td rowspan="6"></td><td>5</td><td>RO</td><td>1 bit</td><td colspan="2">R/W
If the error is a page fault, this indicates whether the faulting access was a read or a write.
0: The faulting access was a read.
1: The faulting access was a write.
Page faults are indicated by error codes 0x03, 0x04, 0x06, 0x1a, and 0xff. For other error code values, this field is unused.</td></tr><tr><td>4</td><td>RO</td><td>1 bit</td><td colspan="2">Batch Member
0: The descriptor was submitted directly.
1: The descriptor was submitted in a batch.</td></tr><tr><td>3</td><td>RO</td><td>1 bit</td><td colspan="2">WQ Index Valid
0: The WQ that the descriptor was submitted to is unknown.
The WQ Index field is unused.
1: The WQ Index field indicates which WQ the descriptor was submitted to.</td></tr><tr><td>2</td><td>RO</td><td>1 bit</td><td colspan="2">Descriptor Valid
0: The descriptor that caused the error is unknown. The Batch Member, Operation, Batch Index, Priv, and PASID fields are unused.
1: The Batch Member, Operation, Batch Index, Priv, and PASID fields are valid.</td></tr><tr><td>1</td><td>RWIC</td><td>1 bit</td><td colspan="2">Overflow
0: The last error recorded in this register is the most recent error.
1: One or more additional errors occurred after the last one recorded in this register.
This field is not cleared by hardware, except by reset. It is cleared by software writing 1.</td></tr><tr><td>0</td><td>RWIC</td><td>1 bit</td><td colspan="2">Valid
0: No error is recorded. All of the other fields of the SWERROR register except Overflow are undefined.
1: An error has occurred and is recorded in this register.
This field is not cleared by hardware, except by reset. It is cleared by software writing 1.</td></tr><tr><td rowspan="3">15:8</td><td rowspan="3">63:32</td><td rowspan="3">RO</td><td rowspan="3">32 bits</td><td colspan="2">Error Information
This field reports additional information for the error codes listed below. Otherwise, this field is unused.</td></tr><tr><td>Error code</td><td>Error information</td></tr><tr><td>Invalid
Flags (0x11)</td><td>63:32 - A bitmask of the flags that were found to be invalid. If a bit in this field is 1, it indicates that the flag at the corresponding bit position in the Flags field of the descriptor was invalid.</td></tr><tr><td colspan="6">SWERROR
Base: BARO
Offset: 0xC0
Size: 32 bytes (4 × 64 bits)</td></tr><tr><td>Byte offset</td><td>Bits</td><td>Attr</td><td>Size</td><td colspan="2">Description</td></tr><tr><td rowspan="6"></td><td rowspan="4"></td><td rowspan="4"></td><td rowspan="4"></td><td>Invalid
Handle
(0x19)</td><td>63:48 – Unused.
47:32 – Interrupt handle.</td></tr><tr><td>Page Fault
(0x03, 0x04)</td><td colspan="1">63:61 – Operand Identifier. See Table 8-13 for a description of this field.
For Inter-domain operations:
60:48 – Unused.
47:32 – The IDPT handle used with the faulting address.
For other operation types, bits 60:32 are unused.</td></tr><tr><td>Page Fault
(0x06, 0x1f)</td><td colspan="1">63:61 – Operand Identifier. See Table 8-13 for a description of this field.
60:32 – Unused.</td></tr><tr><td>Inter-
Domain
Operation
Error
(0x29-
0x2c)</td><td colspan="1">63:48 – Unused.
47:32 – The IDPT handle that caused the error.</td></tr><tr><td>31:16</td><td>RO</td><td>16 bits</td><td colspan="2">Unused.</td></tr><tr><td>15:0</td><td>RO</td><td>16 bits</td><td colspan="2">Batch Index
If the Descriptor Valid field is 1 and the Batch Member field is 1, this field contains the index of the descriptor within the batch.
Otherwise, this field is unused.</td></tr><tr><td>23:16</td><td>63:0</td><td>RO</td><td>64 bits</td><td colspan="2">Address
If the error is a page fault, this is the faulting address. Bits 11:0 may be reported as 0.
Otherwise, this field is undefined.</td></tr><tr><td>31:24</td><td>63:0</td><td>RO</td><td>64 bits</td><td colspan="2">Unused.</td></tr></table>

# 9.2.16 Event Log Configuration Register (EVLCFG)

The Event Log Configuration register is used to configure and manage the region of memory used by the hardware to report several types of error events. Section 5.9 describes the functional details of the event log. The size of event log entries is indicated by the Event Log Support field in GENCAP.

This register is read-write while the device is Disabled and read-only otherwise.

<table><tr><td colspan="4">EVLCFG
Base: BAR0 Offset: 0xE0 Size: 16 bytes (2*64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>127:100</td><td>RSVD</td><td>28 bits</td><td>Reserved.</td></tr><tr><td>99:80</td><td>RWL</td><td>20 bits</td><td>PASID
If PASID Enable is 1, this field specifies the PASID value used for writes to the event log.</td></tr><tr><td>79:64</td><td>RWL</td><td>16 bits</td><td>Event Log Size
Indicates the number of entries in the event log. The maximum number of entries that may be written to the event log without overflow is this value minus 1.
If Event Log Enable is 1, the following constraints apply:
- Event Log Size ≥ 64.
- Event Log Base Address + Event Log Size × Event Log entry size ≤ 264.</td></tr><tr><td>63:12</td><td>RWL</td><td>52 bits</td><td>Event Log Base Address
Base address of the event log. The address must be 4KB aligned.</td></tr><tr><td>11:2</td><td>RSVD</td><td>10 bits</td><td>Reserved.</td></tr><tr><td>1</td><td>RWL</td><td>1 bit</td><td>Priv
The Priv flag used for writes to the event log.
0: The Priv flag is 0 on event log writes.
1: The Priv flag is 1 on event log writes.
This field is reserved if PASID Enable is 0 or if the Privileged Mode Enable field of the PCI Express PASID capability is 0.</td></tr><tr><td>0</td><td>RWL</td><td>1 bit</td><td>PASID Enable
Indicates whether PASID is used for writes to the event log.
0: PASID is not used for event log writes.
1: PASID is used for event log writes. The PASID value is specified by the PASID field in this register.
This field is reserved when the PASID Enable field of the PCI Express PASID capability is 0.</td></tr></table>

# 9.2.17 Event Log Status Register(EVLSTATUS)

If Event Log Enabled in GENCFG is 1, this register is cleared upon successful completion of the Enable Device command.

<table><tr><td colspan="4">EVLSTATUS
Base: BAR0
Offset: 0xF0
Size: 8 bytes (64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63</td><td>RSVD</td><td>1 bit</td><td>Reserved.</td></tr><tr><td>62</td><td>RWIC</td><td>1 bit</td><td>Interrupt Pending
Indicates that an event log interrupt has been generated. Hardware sets this bit when generating an event log interrupt. No further interrupts are generated for additional log entries while this bit is set. Software clears it after processing the interrupt by writing a 1 to this bit.</td></tr><tr><td>61:48</td><td>RSVD</td><td>14 bits</td><td>Reserved.</td></tr><tr><td>47:32</td><td>RO</td><td>16 bits</td><td>Event Log Tail
Index of the event log entry to be written next by hardware. Hardware updates this register after each event written to the event log.</td></tr><tr><td>31:16</td><td>RSVD</td><td>16 bits</td><td>Reserved.</td></tr><tr><td>15:0</td><td>RW</td><td>16 bits</td><td>Event Log Head
Index of the event log entry to be processed next by software. Software updates this register after processing one or more event log entries.</td></tr></table>

# 9.2.18 Inter-Domain Capabilities Register (IDCAP)

The IDCAP register describes capabilities related to inter-domain operations support in the device. This register is present if Inter-Domain support in GENCAP is 1.

<table><tr><td colspan="4">IDCAP
Base: BAR0
Offset: 0x100
Size: 8 bytes (64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:28</td><td>RO</td><td>36 bits</td><td>Reserved.</td></tr><tr><td>27:26</td><td>RO</td><td>2 bits</td><td>IDPTE Size
Indicates the size of an IDPTE. The size of each IDPT entry is 2N+5 bytes, where N is the value in this field.</td></tr><tr><td>25</td><td>RO</td><td>1 bit</td><td>Update Window Suppress Drain Support
0: Suppress Drain flag is not supported in the Update Window descriptor.
1: Software may set the Suppress Drain flag to avoid an implicit drain after an Update Window operation.</td></tr><tr><td>24</td><td>RO</td><td>1 bit</td><td>Offset Mode Support
0: Window Mode field in an IDPTE does not allow selection of Offset Mode.
1: Window Mode field in an IDPTE allows selection of Offset Mode.</td></tr><tr><td>23:8</td><td>RO</td><td>16 bits</td><td>Inter-Domain Permissions Table Size
The number of entries in the Inter-Domain Permissions Table. If this field is 0, there is no Inter-Domain Permissions Table on the device.</td></tr><tr><td>7:2</td><td>RO</td><td>6 bits</td><td>Reserved.</td></tr><tr><td>1:0</td><td>RO</td><td>2 bits</td><td>Inter-Domain Permissions Table Entry Type Support
Bitmask to specify the types of Inter-Domain Permissions Table entries supported by the device. If a bit is 1, then that type is supported.</td></tr></table>

# 9.2.19 Inter-Domain Bitmap Register (IDBR)

If the Inter-Domain Support field in GENCAP is 1, and bit 1 is set in the Type Support field in IDCAP, the Inter-Domain Bitmap register is used to specify the PASID and Privilege to be used to read bitmaps referenced by the IDPT. Otherwise, this register is reserved. This register is read-write while the device is disabled and read-only otherwise.

<table><tr><td colspan="4">IDBR
Base: BARO
Offset: 0x108
Size: 4 bytes (32 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:28</td><td>RSVD</td><td>4 bits</td><td>Reserved.</td></tr><tr><td>27:8</td><td>RWL</td><td>20 bits</td><td>Bitmap PASID
PASID value used to read the bitmap from memory.</td></tr><tr><td>7:5</td><td>RSVD</td><td>3 bits</td><td>Reserved.</td></tr><tr><td>4:2</td><td>RWL</td><td>3 bits</td><td>Bitmap TC
Specifies the traffic class to use to read the bitmap from memory.</td></tr><tr><td>1</td><td>RWL</td><td>1 bit</td><td>Priv
The privilege field used to read the bitmap from memory.
This field is reserved if PASID Enable is 0 or if the Privileged Mode Enable field of the PCI Express PASID capability is 0.</td></tr><tr><td>0</td><td>RWL</td><td>1 bit</td><td>PASID Enable
0: Reads of the bitmap will not be tagged with PASID.
1: Reads of the bitmap will use the PASID specified in this register.
If the PCI Express PASID Capability Enable is 0, this field is reserved.</td></tr></table>

# 9.2.20 Command Parameter Register (CMDPARAM)

The Command Parameter register is used to specify additional parameters for administrative commands. The use of this register depends on the specific command being issued. See section 9.2.12 for details of commands that use the CMDPARAM register. Hardware may only implement the bits in the register that are used by commands supported in CMDCAP. Software must not rely on the value of unimplemented bits. Read of unimplemented bits may return 0 and writes to unimplemented bits may be dropped.

<table><tr><td colspan="4">CMDPARAM
Base: BAR0
Offset: 0xE000</td><td>Size: 8 bytes (64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td><td></td></tr><tr><td>63:0</td><td>RW</td><td>64 bits</td><td>Command Parameters</td><td></td></tr></table>

# 9.2.21 Dummy Portal (DUMMY)

The Dummy Portal behaves like a portal for a WQ that is not enabled. For all addresses on the page, writes are ignored (returning Retry for DMWr) and reads return either 00 or FF for all bytes. See section 9.2.31 for more information about portals. See section 7.3 for how this register may be used for virtualization.

<table><tr><td colspan="2">DUMMY
Base: BAR0
Offset: 0xF000
Size: 0x1000 bytes</td></tr><tr><td>Size</td><td>Description</td></tr><tr><td>0x1000 bytes</td><td>Dummy Portal
Writes are ignored (returning Retry for DMWr) and reads return either 00 or FF for all bytes.</td></tr></table>

# 9.2.22 MSI-X Permissions Table (MSIXPERM)

The MSI-X Permissions Table is a set of 4-byte registers in BARO with the same number of entries as the MSI-X Table. The offset of the MSI-X Permissions Table is given by the MSI-X Permissions Offset field in the Table Offsets register. The number of entries is given by the PCIe-defined MSI-X capability. The individual registers in the table are on 8-byte boundaries.

Each register in the MSI-X Permissions Table corresponds to an entry in the MSI-X table and contains controls associated with that interrupt table entry. These controls are the same as those in the IMS, but these fields cannot be added to the MSI-X table itself, because it is defined by PCIe.

<table><tr><td colspan="4">MSIXPERM
Base: BARO
Offset: Table-offset + index × 8
Size: 4 bytes (32 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:12</td><td>RW</td><td>20 bits</td><td>PASID
If PASID Enable is 1, this field is checked against the PASID field of the descriptor. See section 5.4.</td></tr><tr><td>11:4</td><td>RSVD</td><td>8 bits</td><td>Reserved.</td></tr><tr><td>3</td><td>RW</td><td>1 bit</td><td>PASID Enable
This field is checked against the WQ PASID Enable field of the WQ the descriptor was submitted to. See section 5.4.</td></tr><tr><td>2</td><td>RW</td><td>1 bit</td><td>Ignore
If this field is 1 when a descriptor completion interrupt references the corresponding MSI-X table entry, no interrupt is generated and the Pending field is not set.
This field does not prevent delivery of an interrupt if Pending is 1 and Mask is cleared.
This field does not affect delivery of interrupts due to causes other than descriptor completion.</td></tr><tr><td>1:0</td><td>RSVD</td><td>2 bits</td><td>Reserved.</td></tr></table>

# 9.2.23 Group Configuration Table (GRPCFG)

The Group Configuration Table is an array of registers in BARO that controls the mapping of work queues to engines. The offset of the Group Configuration Table is given by the Group Configuration Offset field in the Table Offsets register. The number of groups is given by the Number of Groups field in GRPCAP. Software may configure the number of groups that it needs. Group Configuration registers beyond the number of groups available are reserved and may not be implemented in hardware.

Each active group contains one or more work queues and one or more engines. Any unused group must have both the WQs field and the Engines field equal to 0. Descriptors submitted to any WQ in a group may be processed by any engine in the group. Each active work queue must be in a single group. (An active work queue is one for which the WQ Size field of the corresponding WQCFG register is non-zero.) Any engine that is not in a group is inactive. See section 3.4 for more information on engines and groups.

Each GRPCFG register is divided into three sub-registers.

These registers are read-write while the device is Disabled and read-only otherwise. They are read-only at all times if the Configuration Support field in GENCAP is 0.

<table><tr><td colspan="2">GRPWQCFG
Base: BARO</td><td colspan="2">Offset: Table-offset + Group-ID × 64 + 0</td><td>Size: 256 bits (4 × 64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td colspan="2">Description</td></tr><tr><td>255:0</td><td>RWL</td><td>256 bits</td><td colspan="2">WQs
Each bit corresponds to a WQ and indicates that the corresponding WQ is in the group. Bits beyond the number of WQs available are reserved and may not be implemented in hardware. Each active WQ must be in exactly one group. Inactive WQs (those for which WQ Size is 0 in WQCFG) must not be in any group.</td></tr></table>

<table><tr><td colspan="4">GRPENGCFG
Base: BARO
Offset: Table-offset + Group-ID × 64 + 32
Size: 8 bytes (64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:0</td><td>RWL</td><td>64 bits</td><td>Engines
Each bit corresponds to an engine and indicates that the corresponding engine is in the group. Bits beyond the number of engines available are reserved and may not be implemented in hardware.</td></tr><tr><td colspan="4">GRPFLAGS
Base: BAR0 Offset: Table-offset + Group-ID × 64 + 40 Size: 8 bytes (64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:48</td><td>RSVD</td><td>16 bits</td><td>Reserved.</td></tr><tr><td>47:44</td><td>RWL</td><td>4 bits</td><td>Write Bandwidth LimitThis field controls the aggregate maximum allowable write bandwidth for all engines in the group as a fraction of the maximum write bandwidth supported by the device implementation. This field is reserved if the Bandwidth Limit Support field is GRPCAP is 0. 0: No limit. Each engine is allowed up to the maximum write bandwidth that it is capable of. 1: Limit the total write bandwidth of the group to 1/2 of the maximum value. 2: Limit the total write bandwidth of the group to 1/4 of the maximum value. 3: Limit the total write bandwidth of the group to 1/8 of the maximum value. 4-15: Reserved.</td></tr><tr><td>43:40</td><td>RWL</td><td>4 bits</td><td>Read Bandwidth LimitThis field controls the aggregate maximum allowable read bandwidth for all engines in the group as a fraction of the maximum read bandwidth supported by the device implementation. This field is reserved if the Bandwidth Limit Support field is GRPCAP is 0. 0: No limit. Each engine is allowed up to the maximum read bandwidth that it is capable of. 1: Limit the total read bandwidth of the group to 1/2 of the maximum value. 2: Limit the total read bandwidth of the group to 1/4 of the maximum value. 3: Limit the total read bandwidth of the group to 1/8 of the maximum value. 4-15: Reserved.</td></tr><tr><td>39:38</td><td>RSVD</td><td>2 bits</td><td>Reserved.</td></tr><tr><td>37:36</td><td>RWL</td><td>2 bits</td><td>Batch Descriptors in Progress LimitThis field controls the number of batch descriptors that can be concurrently processed by an engine in the group as a fraction of the Maximum Batch Descriptors in Progress value specified in ENGCAP. Note that exercising this control can cause bandwidth to be reduced in some cases, depending on prevailing system latency. This field is reserved if the Descriptors in Progress Limit Supported field is GRPCAP is 0. 00: Allow up to the maximum value that an engine is capable of. 01: Limit to 1/2 of the maximum value. 10: Limit to 1/4 of the maximum value. 11: Limit to 1/8 of the maximum value.</td></tr><tr><td>35:34</td><td>RSVD</td><td>2 bits</td><td>Reserved.</td></tr><tr><td>33:32</td><td>RWL</td><td>2 bits</td><td>Work Descriptors in Progress LimitThis field controls the number of work descriptors that can be concurrently processed by an engine in the group as a fraction of the Maximum Work Descriptors in Progress value specified in ENGCAP.Note that exercising this control can cause bandwidth to be reduced in some cases, depending on prevailing system latency.This field is reserved if the Descriptors in Progress Limit Supported field is GRPCAP is 0.00: Allow up to the maximum value that an engine is capable of.01: Limit to \(1/2\)of the maximum value.10: Limit to \(1/4\)of the maximum value.11: Limit to \(1/8\)of the maximum value.</td></tr><tr><td>31:28</td><td>RSVD</td><td>4 bits</td><td>Reserved.</td></tr><tr><td>27:20</td><td>RWL</td><td>8 bits</td><td>Read Buffers AllowedThis field indicates the maximum number of Read Buffers that may be in use at one time by all engines in the group. This value can be used to limit the maximum bandwidth used by engines in the group.This value must be:- greater than or equal to 4 times the number of engines in the group;- greater than or equal to the Read Buffers Reserved field for this group; and- less than or equal to the sum of the Read Buffers Reserved field and the number of non-reserved Read Buffers.(The number of non-reserved Read Buffers is the Total Read Buffers field in GRPCAP minus the total of the Read Buffers Reserved fields for all groups.)The default value of this field is the same as the value of the Total Read Buffers field in GRPCAP.If the Read Buffer Controls Supported field in GRPCAP is 0, this field is read-only and is unused.</td></tr><tr><td>19:16</td><td>RSVD</td><td>4 bits</td><td>Reserved.</td></tr><tr><td>15:8</td><td>RWL</td><td>8 bits</td><td>Read Buffers ReservedThis field indicates the number of Read Buffers reserved for the use of engines in the group. This value can be used to reduce the possibility of contention with engines in other groups. However, if it is set to a non-zero value, it may reduce the overall performance of the device. The sum of the Read Buffers reserved for all groups must be less than or equal to the Total Read Buffers field in GRPCAP.If the Read Buffer Controls Supported field in GRPCAP is 0, this field is read-only and is unused.</td></tr><tr><td>7</td><td>RWL</td><td>1 bit</td><td>Use Global Read Buffer Limit0: The Global Read Buffer Limit does not apply to this group.1: The Global Read Buffer Limit programmed in the GENCFG register applies to descriptors processed by engines in this group. (The limit indicated by the Read Buffers Allowed field applies as well.)If the Global Read Buffer Limit Supported field in GRPCAP is 0, this field is reserved.</td></tr><tr><td>6</td><td>RSVD</td><td>1 bit</td><td>Reserved.</td></tr><tr><td>5:3</td><td>RWL</td><td>3 bits</td><td>TC-B
Specifies the traffic class to use for memory accesses for which the traffic class selector in the descriptor is 1.</td></tr><tr><td>2:0</td><td>RWL</td><td>3 bits</td><td>TC-A
Specifies the traffic class to use for memory accesses for which the traffic class selector in the descriptor is 0.</td></tr></table>

# 9.2.24 WQ Configuration Table (WQCFG)

The WQ Configuration Table is an array of registers in BAR0. The offset of the WQ Configuration Table is given by the WQ Configuration Offset field in the Table Offsets register. The number of WQs is given by the Number of WQs field in WQCAP. The size of the WQCFG register for each WQ is given by the WQCFG Size field in WQCAP. The size is  $2^{N + 5}$  bytes, where N is the value of the WQCFG Size field.

Each WQCFG register is divided into sub-registers, which may be read or written using aligned 1-, 2-, 4-, or 8-byte read or write operations. The fields of WQCFG are read-only or read-write at different times, depending on device state, WQ state, the Configuration Support field in GENCAP, and the WQ Mode Support field, as detailed in the table. Any writes to fields while they are read-only are ignored.

<table><tr><td rowspan="3">Field</td><td colspan="3">Configuration Support</td></tr><tr><td rowspan="2">1</td><td colspan="2">0</td></tr><tr><td>Mode Support=0</td><td>Mode Support=1</td></tr><tr><td>Mode Support</td><td colspan="3">Read-only at all times</td></tr><tr><td>Size</td><td>Read-write while device is Disabled; read-only otherwise</td><td>Read-only at all times</td><td>Read-only at all times</td></tr><tr><td>Threshold</td><td>Read-write at all times</td><td>Read-only at all times</td><td>Read-write at all times</td></tr><tr><td>Mode Priv PASID Enable PASID</td><td>Read-write while WQ is Disabled; read-only otherwise</td><td>Read-only at all times</td><td>Read-write while WQ is Disabled; read-only otherwise</td></tr><tr><td>Priority Block-on-Fault Enable Maximum Transfer Size Maximum Batch Size ATS Disable PRS Disable</td><td>Read-write while WQ is Disabled; read-only otherwise</td><td>Read-only at all times</td><td>Read-only at all times</td></tr><tr><td>Occupancy Interrupt Enable</td><td>Read-write at all times</td><td>Read-only at all times</td><td>Read-write at all times</td></tr><tr><td>Occupancy Limit</td><td>Read-only while Occupancy Interrupt Enable is 1</td><td>Read-only at all times</td><td>Read-only while Occupancy Interrupt Enable is 1</td></tr><tr><td>Occupancy Interrupt Table Occupancy Interrupt Handle</td><td>Read-write while WQ is Disabled; read-only otherwise</td><td>Read-only at all times</td><td>Read-write while WQ is Disabled; read-only otherwise</td></tr><tr><td>Operations Configuration</td><td>Read-write while WQ is Disabled; read-only otherwise</td><td>Read-only at all times</td><td>Read-only at all times</td></tr></table>

Table 9-7: Work Queue Configuration Support

The WQ Size fields of all the WQCFG registers must be set before the device is enabled. The sum of all the WQ Size fields must not be greater than Total WQ Size field in WQCAP. WQs for which the WQ Size field is 0 are inactive and cannot be enabled. The other configuration fields for inactive WQs are ignored.

At the time a WQ is enabled, consistency checks are performed on the fields of the WQCFG register. See section 5.2 for the checks that are performed.

<table><tr><td colspan="5">WQCFG
Base: BAR0 Offset: Table-offset + WQ-ID × WQCFG-Size Size: WQCFG-Size bytes</td></tr><tr><td>Bytes</td><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td rowspan="2">3:0</td><td>31:16</td><td>RSVD</td><td>16 bits</td><td>Reserved.</td></tr><tr><td>15:0</td><td>1</td><td>16 bits</td><td>WQ Size
The number of entries in the WQ storage allocated to this WQ.
The sum of the WQ Size fields for all work queues must be less than or equal to the Total WQ Size field in WQCAP.</td></tr><tr><td rowspan="2">7:4</td><td>31:16</td><td>RSVD</td><td>16 bits</td><td>Reserved.</td></tr><tr><td>15:0</td><td>1</td><td>16 bits</td><td>WQ Threshold
The number of entries in this WQ that may be filled via a limited portal. If WQ Occupancy is greater than or equal to WQ Threshold, work submissions using a limited portal return Retry.
The threshold applies only to shared work queues. If WQ Mode is 1 (dedicated mode), this field is ignored. If WQ Threshold is greater than WQ Size, it is treated as if it is equal to WQ Size.</td></tr><tr><td rowspan="5">11:8</td><td>31:30</td><td>RSVD</td><td>2 bits</td><td>Reserved.</td></tr><tr><td>29</td><td>1</td><td>1 bit</td><td>WQ Priv
The Priv flag to be used for descriptors submitted to this WQ when it is in dedicated mode.
If the WQ is in dedicated mode, WQ PASID Enable is 1, and the Privileged Mode Enable field of the PCI Express PASID capability is 0, this field must be 0.
If the WQ is in shared mode or WQ PASID Enable is 0, this field is ignored.</td></tr><tr><td>28</td><td>1</td><td>1 bit</td><td>WQ PASID Enable
Indicates whether PASID is used for address translation requests for descriptors from this WQ.
If the PCI Express PASID capability is not enabled, this field must be 0.
If WQ Mode is 0 (SWQ), this field must be 1.</td></tr><tr><td>27:8</td><td>1</td><td>20 bits</td><td>WQ PASID
The PASID to be used for descriptors submitted to this WQ when it is in dedicated mode. If the WQ is in shared mode or WQ PASID Enable is 0, this field is ignored.</td></tr><tr><td>7:4</td><td>1</td><td>4 bits</td><td>WQ Priority
If the WQ Priority Support field in WQCAP is 1, this field indicates the priority of this work queue relative to other WQs in the same group. This field must not be 0. See section 4.1 for a description of WQ priorities.
If the WQ Priority Support field in WQCAP is 0, this field is ignored.</td></tr><tr><td rowspan="4"></td><td>3</td><td>1</td><td>1 bit</td><td>WQ PRS Disable
0: PRS is used for descriptors submitted to this WQ according to the setting of the Enable field in the PCIe PRS capability, the WQ Block on Fault Enable field, and the Block on Fault flag in the descriptor.
1: PRS is not used for descriptors submitted to this WQ even when the Enable field in the PCIe PRS capability is 1. The WQ Block on Fault Enable field must be 0.
If WQ PRS Support is 0, this field is reserved and may be hardwired to 0.
If Event Log Enable in GENCFG is 0, this field must be 0.</td></tr><tr><td>2</td><td>1</td><td>1 bit</td><td colspan="1">WQ ATS Disable
0: ATS is used for descriptors submitted to this WQ according to the setting of the Enable field in the PCIe ATS capability.
1: ATS is not used for descriptors submitted to this WQ even when the Enable field in the PCIe ATS capability is 1.
If WQ ATS Support is 0, this field is reserved and may be hardwired to 0.</td></tr><tr><td>1</td><td>1</td><td>1 bit</td><td colspan="1">WQ Block on Fault Enable
0: Block on fault is not allowed. The Block On Fault flag in descriptors submitted to this WQ is reserved. If a page fault occurs on a source or destination memory access, the operation stops and the page fault is reported to software.
1: Block on fault is allowed. Behavior on page faults depends on the values of the Block on Fault flag in each descriptor.
This field is reserved if the Block on Fault Support field in GENCAP is 0 or if the Enable field of the PCIe Page Request Control Register is 0.</td></tr><tr><td>0</td><td>1</td><td>1 bit</td><td colspan="1">WQ Mode
0: WQ is in shared mode.
1: WQ is in dedicated mode.</td></tr><tr><td colspan="5">WQCFG
Base: BARO Offset: Table-offset + WQ-ID × WQCFG-Size Size: WQCFG-Size bytes</td></tr><tr><td>Bytes</td><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td rowspan="4">15:12</td><td>31:13</td><td>RSVD</td><td>19 bits</td><td>Reserved.</td></tr><tr><td>12:9</td><td>1</td><td>4 bits</td><td colspan="1">WQ Maximum SGL Size
The maximum number of entries in a Scatter Gather List referenced by a Scatter or Gather descriptor submitted to this WQ is 2N, where N is the value in this field.
If Scatter Gather operations are not supported (as indicated by corresponding bits in OPCAP), this field is reserved. If supported, this field must not be greater than the Maximum Supported SGL Size field in DSACAP0. Software should set this field to the minimum size needed, to limit how long descriptors can block other descriptors behind them.</td></tr><tr><td>8:5</td><td>1</td><td>4 bits</td><td colspan="1">WQ Maximum Batch Size
The maximum number of descriptors that can be referenced by a Batch descriptor submitted to this WQ is 2N, where N is the value in this field.
If Batch descriptor is supported (as indicated by bit 1 of OPCAP), this field must not be 0 and must not be greater than the Maximum Supported Batch Size field in GENCAP.
Otherwise, this field is reserved. It is checked when the WQ is enabled.
Software should set this field to the minimum size needed, to limit how long descriptors can block other descriptors behind them.</td></tr><tr><td>4:0</td><td>1</td><td>5 bits</td><td colspan="1">WQ Maximum Transfer Size
The maximum transfer size that can be specified in a descriptor submitted to this WQ is 2N, where N is the value in this field.
This field must not be greater than the Maximum Supported Transfer Size field in GENCAP. It is checked when the WQ is enabled.
Software should set this field to the minimum size needed, to limit how long descriptors can block other descriptors behind them.</td></tr><tr><td rowspan="2">19:16</td><td>31:17</td><td>RSVD</td><td>15 bits</td><td>Reserved.</td></tr><tr><td>16</td><td>RWL</td><td>1 bit</td><td colspan="1">WQ Occupancy Interrupt Table
0: WQ Occupancy Interrupt Handle is a handle for the MSI-X table.
1: WQ Occupancy Interrupt Handle is a handle for the IMS. This field is read-only except while the WQ is Disabled.
If the Interrupt Message Storage Size field in GENCAP is 0, this field must be 0.
If the WQ Occupancy Interrupt Support field in WQCAP is 0, this field is reserved.</td></tr><tr><td></td><td>15:0</td><td>RWL</td><td>16 bits</td><td>WQ Occupancy Interrupt Handle
An interrupt handle indicating which interrupt table entry to use to generate the interrupt.
When the Interrupt Handle Request capability is 0, this field is the index of the desired entry in the MSI-X table or IMS.
When the Interrupt Handle Request capability is 1, this is a handle returned by the Request Interrupt Handle command.
This field is read-only except while the WQ is Disabled.
If the WQ Occupancy Interrupt Support field in WQCAP is 0, this field is reserved.</td></tr><tr><td rowspan="3">23:20</td><td>31:17</td><td>RSVD</td><td>15 bits</td><td>Reserved.</td></tr><tr><td>16</td><td>RW</td><td>1 bit</td><td colspan="1">WQ Occupancy Interrupt Enable
Setting this field to 1 causes the device to generate an interrupt when the WQ occupancy is at or less than the WQ Occupancy Limit. The device sets the Interrupt Generated field when the interrupt is generated. This field may be set to 1 at the same time the WQ Occupancy Limit field is set to the desired value.
If this field is set to 1 with Limit ≥ the current WQ occupancy, the interrupt is generated immediately.
If the WQ Occupancy Interrupt Support field in WQCAP is 0, this field is reserved.
If this field is set to 1 while the WQ is Disabled, the interrupt will be delivered at the time the WQ is enabled.</td></tr><tr><td>15:0</td><td>RWL</td><td>16 bits</td><td colspan="1">WQ Occupancy Limit
When the WQ Occupancy Interrupt Enable is 1 and the WQ occupancy is at or below the value in this field, an interrupt is generated.
This field is read-only while WQ Occupancy Interrupt Enable is 1; however, it may be changed at the same time that WQ Occupancy Interrupt Enable is set to 1.
If the WQ Occupancy Interrupt Support field in WQCAP is 0, this field is reserved.</td></tr><tr><td colspan="5">WQCFG
Base: BAR0 Offset: Table-offset + WQ-ID × WQCFG-Size Size: WQCFG-Size bytes</td></tr><tr><td>Bytes</td><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td rowspan="5">27:24</td><td>31:30</td><td>RO</td><td>2 bits</td><td>WQ State
00: WQ is Disabled. Descriptors are not accepted into the WQ.
(ENQ operations to this WQ return Retry. Other write operations are ignored.)
01: WQ is Enabled. Descriptors may be submitted and processed.
10: Disable WQ, Reset WQ, Disable Device, or Reset Device command is in progress. Descriptors are not accepted into the WQ. Descriptors currently in the WQ are being drained.
11: Unused.</td></tr><tr><td>29</td><td>RO</td><td>1 bit</td><td colspan="1">WQ Mode Support
When the Configuration Support field in GENCAP is 0, this field indicates whether certain WQ configuration fields are read-only. See the table in this section for the meaning of this field. When the Configuration Support field in GENCAP is 1, this field is unused.</td></tr><tr><td>28:17</td><td>RSVD</td><td>12 bits</td><td colspan="1">Reserved.</td></tr><tr><td>16</td><td>RWIC</td><td>1 bit</td><td colspan="1">WQ Occupancy Interrupt Generated
0: There are no WQ Occupancy Interrupts for this WQ that have not been acknowledged by software. Device is able to generate a WQ Occupancy Interrupt if the conditions are satisfied.
1: WQ Occupancy Interrupt was generated. Software should write a 1 to clear this bit. This bit must be cleared before another WQ Occupancy Interrupt can be generated for this WQ.</td></tr><tr><td>15:0</td><td>RO</td><td>16 bits</td><td colspan="1">WQ Occupancy
The number of entries currently in this WQ. This number may change whenever descriptors are submitted to or dispatched from the queue, so it cannot be relied on to determine whether there is space in the WQ.
If the WQ Occupancy Support field in WQCAP is 0, the value in this field is undefined.</td></tr><tr><td>31:28</td><td>31:0</td><td>RSVD</td><td>32 bits</td><td>Reserved.</td></tr><tr><td>63:32</td><td>255:0</td><td>1</td><td>256 bits</td><td>WQ Operations Configuration
Each bit corresponds to an operation code and indicates whether that operation type is allowed in the WQ. If the bit is 1, the corresponding operation type is allowed to be used in the WQ; if the bit is 0, the corresponding operation type is not allowed to be used in the WQ. Bits corresponding to undefined operation codes are unused and are read as 0.</td></tr></table>

# 9.2.25 Performance Monitoring Registers

The performance monitoring registers are a collection of registers in BARO to discover capabilities, configure and control the performance monitoring capabilities in Intel DSA. The capability registers include a global performance monitoring capability register (PERFCAP) and registers to describe per-event category (EVNTCAP) and optionally, per-counter (CNTRCAP) capabilities.

<table><tr><td>Perfmon Register</td><td>Conditions Under Which Register is Read-Only</td></tr><tr><td>CNTRCFG</td><td>All fields except Enable are read-only while the counter is enabled.</td></tr><tr><td>FLTCFG</td><td>Read-only while corresponding counter is enabled.</td></tr><tr><td>CNTRDATA</td><td>Read-only while corresponding counter is enabled, if the Counters Writeable while Enabled field in PERFCAP is 0.</td></tr><tr><td>PERFRST</td><td>Read-Write at all times.</td></tr><tr><td>OVFSTATUS</td><td>Read-Write at all times.</td></tr><tr><td>PERFFRZ</td><td>Read-Write at all times.</td></tr></table>


Table 9-8: Perfmon Register Read-Only Status


# 9.2.25.1 Performance Monitoring Capabilities Register (PERFCAP)

<table><tr><td colspan="4">PERFCAP
Base: BARO
Offset: Table-Offset
Size: 8 bytes (64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:56</td><td>RO</td><td>8 bits</td><td>Unused</td></tr><tr><td>55</td><td>RO</td><td>1 bit</td><td>Interrupt on Overflow Support
0: Device does not support generation of interrupts upon counter overflow.
1: Device supports generation of interrupt upon counter overflow.
Interrupt generation is controlled by the Interrupt on Overflow bit in the CNTRCFG registers.</td></tr><tr><td>54</td><td>RO</td><td>1 bit</td><td>Counter Freeze Support
0: Counter Freeze controls in PERFFRZ and CNTRCFG registers are not supported.
1: Counter Freeze controls in PERFFRZ and CNTRCFG are supported.</td></tr><tr><td>53</td><td>RO</td><td>1 bit</td><td>Counters Writeable While Enabled
0: Indicates that software is not allowed to write to a counter data register while that counter is enabled. Counter registers are always writeable while disabled.
1: Indicates that hardware supports writes to a counter data register while it is enabled.</td></tr><tr><td>52</td><td>RO</td><td>1 bit</td><td>Per Counter Capabilities Supported
Indicates whether per counter capability registers are supported.
0: All supported counters have the same capabilities (i.e., can be used to monitor any of the supported events, can be used with any of the filter types etc.) and per counter capability registers are not supported.
1: Software should read the per-counter capability registers to identify the Event Categories, Events and Filters supported by each counter.</td></tr><tr><td>51:44</td><td>RO</td><td>8 bits</td><td>Unused</td></tr></table>

<table><tr><td colspan="2">PERFCAP
Base: BARO</td><td colspan="2">Offset: Table-Offset</td><td>Size: 8 bytes (64 bits)</td></tr><tr><td>43:36</td><td>RO</td><td>8 bits</td><td colspan="2">Filters Supported
Bitmask indicating which Filters are supported in this implementation.
If no filters are supported, then this field is 0. Note that even if this field is non-zero, not all filters may be supported for each Event. See Appendix D for information on which filters are supported for each Event. Table 6-2 describes the details for each of the filters supported. The number of Filter Configuration registers per counter corresponds to the number of bits set to 1 in this field.</td></tr><tr><td>35:20</td><td>RO</td><td>16 bits</td><td colspan="2">Global Event Categories Supported
Bitmask indicating the Event Categories that may be specified with any of the counters. If per-counter capabilities are supported, the value in CNTRCAP overrides the value specified here.</td></tr><tr><td>19:16</td><td>RO</td><td>4 bits</td><td colspan="2">Number of Event Categories Supported
The Event Categories are listed in Table 6-1. The EVNTCAP register corresponding to each supported Event Category indicates the events supported in that category.</td></tr><tr><td>15:8</td><td>RO</td><td>8 bits</td><td colspan="2">Counter Width
The number of bits supported per counter. If the value of this field is n, then each counter is an n-bit counter and the max value it can count is 2n-1. If per-counter capabilities are supported, the counter width specified in the CNTRCAP registers overrides this value.</td></tr><tr><td>7:6</td><td>RO</td><td>2 bits</td><td colspan="2">Unused</td></tr><tr><td>5:0</td><td>RO</td><td>6 bits</td><td colspan="2">Number of Performance Monitoring Counter Registers Supported
A value of 0 indicates that performance counters are not supported.</td></tr></table>

# 9.2.25.2 Performance Monitoring Event Capabilities Register (EVNTCAP)

Each EVNTCAP register corresponds to an Event Category and reports the set of events supported for that Event Category. The number of EVNTCAP registers corresponds to the number of Event Categories reported in PERFCAP. For example, if the number of Event Categories defined is 5, there will be five EVNTCAP registers, namely EVNTCAP_0, EVNTCAP_1 and so on, one for each of the Event Categories.

<table><tr><td colspan="4">EVNTCAP_mmm
Base: BAR0 Offset: Table-Offset + 0x80 + Size: 8 bytes (64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:28</td><td>RSVD</td><td>36 bits</td><td>Reserved.</td></tr><tr><td>27:0</td><td>RO</td><td>28 bits</td><td>Events
Bitmask of events supported for this Event Category. The Event Category that this register corresponds to, depends on the offset of this register. There is a separate EVNTCAP register for each Event Category supported in the implementation.
Any bit that is 1 indicates that the corresponding event is supported. Note that the set of Events supported for any given Event Category is implementation-specific and may change in future implementations. When programming the CNTRCFG register with a particular Event Category value, if software sets Events bits not supported for that Event Category, those bits are ignored.
If the implementation does not support any events for a given Event Category, this field is 0.</td></tr></table>

# 9.2.25.3 Performance Monitoring Counter Capabilities Register (CNTRCAP)

The CNTRCAP registers report the Event Categories and Events allowed for each counter. Implementations which do not have any restrictions on mapping of Event Categories to counters do not support these registers. These registers are present only if the Per Counter Capabilities Supported field in PERFCAP is 1. If present, the number of these capability registers corresponds to the number of Performance monitoring counter registers reported in PERFCAP. The values specified in each capability register apply only to the corresponding counter and override the values specified in PERFCAP.

<table><tr><td colspan="4">CNTRCAP_nnn
Base: BAR0 Offset: Table-Offset + 0x800 + Counter-Num × 64 Size: 64 bytes (512 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>-</td><td>RO</td><td>28 bits</td><td>Events_k</td></tr><tr><td>-</td><td>RO</td><td>4 bits</td><td>Event Category_k</td></tr><tr><td></td><td></td><td></td><td>...</td></tr><tr><td>95:68</td><td>RO</td><td>28 bits</td><td>Events_1</td></tr><tr><td>67:64</td><td>RO</td><td>4 bits</td><td>Event Category_1</td></tr><tr><td>63:36</td><td>RO</td><td>28 bits</td><td>Events_0
Specifies the Events supported in this counter register for the Event Category below.</td></tr><tr><td>35:32</td><td>RO</td><td>4 bits</td><td>Event Category_0
Specifies the first Event Category that can be enabled in this counter register.</td></tr><tr><td>31:28</td><td>RO</td><td>4 bits</td><td>Number of Event Entries
This field indicates the number of records describing the Event Categories and Events supported by this counter register.
For example, if a given counter can only count events corresponding to a single category (e.g., WQ related events), then this field will be 1 and there will be 1 pair of entries reported in the Event Category and Events fields in this register.
If this field is 0, then this counter supports all Global Event Categories specified in PERFCAP.</td></tr><tr><td>27:8</td><td>RO</td><td>20 bits</td><td>Unused</td></tr><tr><td>7:0</td><td>RO</td><td>8 bits</td><td>Counter Width
The value of this field represents the number of bits supported for this counter. If the value of this field is n, then the counter is an n-bit counter and the max value it can count is 2n-1.</td></tr></table>

# 9.2.25.4 Performance Monitoring Reset Control Register (PERFRST)

The PERFRST register can be used by software to reset all the performance monitoring configuration and data registers to their default values.

<table><tr><td colspan="4">PERFIRST
Base: BARO
Offset: Table-Offset + 0x10
Size: 4 bytes (32 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:2</td><td>RSVD</td><td>30 bits</td><td>Reserved.</td></tr><tr><td>1</td><td>RWV</td><td>1 bit</td><td>Reset Perfmon Counters
Software writes a 1 to this bit to reset all the Performance Monitoring Data registers. All the CNTRDATA registers are initialized to 0.
Hardware clears this bit when all the counter data registers have been reset.</td></tr><tr><td>0</td><td>RWV</td><td>1 bit</td><td>Reset Perfmon Configuration
Software writes a 1 to this bit to reset all the Performance Monitoring Configuration registers. All the CNTRCFG, FLTCFG, OVFSTATUS and PERFFRZ registers are initialized to default values. Hardware clears this bit when all the configuration registers have been reset.</td></tr></table>

# 9.2.25.5 Performance Monitoring Overflow Status Register (OVFSTATUS)

OVFSTATUS is a register used to indicate status across all the performance monitoring counters supported. Any bits beyond the number of counters reported in the PERFCAP register will be reported as 0 and should be ignored by software.

<table><tr><td colspan="4">OVFSTATUS
Base: BARO
Offset: Table-Offset + 0x30
Size: 4 bytes (32 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:0</td><td>RWIC</td><td>32 bits</td><td>Overflow Status
Bitmask with 1 bit per counter. Bit N indicates whether performance counter N has encountered an overflow condition.
0: Counter has not encountered an overflow condition.
1: Counter has encountered an overflow condition.
Writing 1 clears the bit.</td></tr></table>

# 9.2.25.6 Performance Monitoring Freeze Register (PERFFRZ)

The PERFFRZ register can be used by software to control the freeze behavior and monitor the freeze status of all the performance monitoring counters. This register is present only if the Counter Freeze Support field in PERFCAP is 1.

<table><tr><td colspan="4">PERFFRZ
Base: BARO
Offset: Table-Offset + 0x20
Size: 4 bytes (32 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:0</td><td>RWV</td><td>32 bits</td><td>Freeze Control and Status
Bitmask with 1 bit per counter.
Writing a 0 or 1 has the following impact on the corresponding counter:
0: The counter is unfrozen and resumes counting unless
CNTRCFG.Enabled=0; in which case the counter remains disabled. If
the counter is enabled but not currently frozen, it is unaffected and
continues to count events.
1: The counter, if enabled, gets frozen and stops counting further
events, and retains its current value. If a counter is already frozen
when this bit is set, it remains frozen.
Reads return the current freeze status of each counter:
0: The counter is currently not frozen. The counter may be disabled
(CNTRCFG.Enabled=0), or may be enabled and counting events.
1: The counter is currently frozen and not counting events. It remains
frozen until explicitly unfrozen by software.
Bits corresponding to counters not supported by the hardware are
ignored. Disabling a counter by setting CNTRCFG.Enabled to 0 clears
the freeze status for that counter.</td></tr></table>

# 9.2.25.7 Counter Configuration Register (CNTRCFG)

The CNTRCFG registers specify the set of events to be monitored by each counter. They also control interrupt generation behavior and the behavior upon overflow. The number of CNTRCFG registers corresponds to the number of counter registers specified in PERFCAP. The default value of these registers is 0. All fields except Enable are read-only while the counter is enabled.

<table><tr><td colspan="4">CNTRCFG_nnn
Base: BAR0
Offset: Table-Offset + 0x100 +
Counter-Num × 8</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:60</td><td>RSVD</td><td>4 bits</td><td>Reserved.</td></tr><tr><td>59:32</td><td>RWL</td><td>28 bits</td><td>Events
Specifies the set of events to be monitored by this counter, corresponding to the Event Category selected. The set of supported events depends on the value of Event Category.Unsupported bits are ignored. The definition of some Events in each Event Category may be implementation specific.</td></tr></table>

<table><tr><td colspan="5">CNTRCFG_nnn
Base: BAR0 Offset: Table-Offset + 0x100 +
Size: 8 bytes (64 bits)</td></tr><tr><td>31:12</td><td>RSVD</td><td>20 bits</td><td colspan="2">Reserved.</td></tr><tr><td>11:8</td><td>RWL</td><td>4 bits</td><td colspan="2">Event Category
Specifies the Event Category to associate with this counter. Based on the Event Category selected, different sets of events can be selected in the Events field.</td></tr><tr><td></td><td></td><td></td><td>Value</td><td>Event Category</td></tr><tr><td></td><td></td><td></td><td>0</td><td>WQ</td></tr><tr><td></td><td></td><td></td><td>1</td><td>Engine</td></tr><tr><td></td><td></td><td></td><td>2</td><td>Address Translation</td></tr><tr><td></td><td></td><td></td><td>3</td><td>Operations</td></tr><tr><td></td><td></td><td></td><td>4</td><td>Completions</td></tr><tr><td></td><td></td><td></td><td>5</td><td>Operations 2</td></tr><tr><td></td><td></td><td></td><td>6-15</td><td>Reserved</td></tr><tr><td>7:3</td><td>RSVD</td><td>5 bits</td><td colspan="2">Reserved.</td></tr><tr><td>2</td><td>RWL</td><td>1 bit</td><td colspan="2">Global Freeze on Overflow
0: No global freeze.
1: When an overflow is detected from this register, all counters in the device are frozen.
In either case, Overflow status is recorded in the OVFSTATUS register.
This bit is reserved if Counter Freeze Support in PERFCAP is 0.</td></tr><tr><td>1</td><td>RWL</td><td>1 bit</td><td colspan="2">Interrupt on Overflow
0: No Interrupt is generated.
1: Generate a Performance monitoring Interrupt when this counter overflows.
This bit is reserved if Interrupt on Overflow Support in PERFCAP is 0.</td></tr><tr><td>0</td><td>RW</td><td>1 bit</td><td colspan="2">Enable
0: This counter is disabled.
1: This counter is enabled to count events.</td></tr></table>

# 9.2.25.8 Filter Configuration Register (FLTCFG)

Each counter supports a set of Filter Configuration registers, one for each filter defined in Table 6-2. Software can program one or more Filter Configuration registers with the filter values to apply to that counter. For example, FLTCFG_WQ_0 selects the WQs to monitor for events in counter 0, FLTCFG_TC_2 selects the Traffic Classes to monitor for events in counter 2, and so on. Each FLTCFG register has a default value of all ls which implies that no constraints are imposed by that filter. Table 9-9 shows an example set of register offsets for the set of Filter Configuration registers associated with each counter. This register is read-only while the corresponding counter is enabled.

<table><tr><td colspan="4">FLTCFG_F_nnn
Base: BAR0
Offset: Table-Offset + 0x300 +
Size: 4 bytes (32 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:16</td><td>RSVD</td><td>16 bits</td><td>Reserved.</td></tr><tr><td>15:0</td><td>RWL</td><td>16 bits</td><td>Filter Value
Specifies the filter value to be used for the Filter associated with this register. It defaults to all ls implying that all values are allowed. Bits beyond the max value allowed for that filter are ignored. For example, for the WQ filter, bits beyond the number of enabled WQs are ignored.</td></tr></table>

<table><tr><td colspan="4">FLTCFG_PASID_nnn
Base: BAR0
Offset: Table-Offset + 0x300 +
Counter-Num × 32 + 0x14</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>31:22</td><td>RSVD</td><td>10 bits</td><td>Reserved.</td></tr><tr><td>21:0</td><td>RWL</td><td>22 bits</td><td>Filter Value
Specifies the filter value to be used for the Filter associated with this register. It defaults to all ls implying that all values are allowed. This field is read-only when the corresponding counter is enabled for counting. Bits beyond the max value allowed for that filter are ignored.</td></tr></table>

<table><tr><td>Filter Configuration Register</td><td>BARO Offset</td></tr><tr><td>FLTCFG_WQ_0</td><td>0x1300</td></tr><tr><td>FLTCFG_TC_0</td><td>0x1304</td></tr><tr><td>FLTCFG_PGSZ_0</td><td>0x1308</td></tr><tr><td>FLTCFG_SZ_0</td><td>0x130C</td></tr><tr><td>FLTCFG_ENG_0</td><td>0x1310</td></tr><tr><td>FLTCFG_PASID_0</td><td>0x1314</td></tr><tr><td>Unused</td><td>0x1318 - 0x131F</td></tr><tr><td>FLTCFG_WQ_1</td><td>0x1320</td></tr><tr><td>FLTCFG_TC_1</td><td>0x1324</td></tr><tr><td>...</td><td>...</td></tr><tr><td>FLTCFG_ENG_N</td><td>0x1300 + N × 0x20 + 0x10</td></tr><tr><td>FLTCFG_PASID_N</td><td>0x1300 + N × 0x20 + 0x14</td></tr></table>

Table 9-9: Filter Configuration Register Offsets

# 9.2.25.9 Counter Data Register (CNTRDATA)

Each CNTRDATA register is an N-bit counter that is used to count occurrences of configured events, where N is the value of the Counter Width field in PERFCAP. Behavior of software reads and writes to these registers are described in section 6.3. Once written, the counter continues to increment from the written value. A freeze operation causes the counter to stop accumulating further events and to retain its value at the time of freeze. An unfreeze operation allows the counter to resume counting subsequent events.

<table><tr><td colspan="4">CNTRDATA_nnn
Base: BAR0
Offset: Table-Offset + 0x200 +
Counter-Num × 8</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:N</td><td>RSVD</td><td>64-N bits</td><td>Ignored.</td></tr><tr><td>N-1:0</td><td>RWLV</td><td>N bits</td><td>Event Count Value
N-bit performance event counter where N is the value of the Counter
Width field in PERFCAP.
If the Counters Writeable while Enabled field in PERFCAP is 0, then this register is read-only while the counter is Enabled.</td></tr></table>

# 9.2.26 MSI-X Table

BAR0; Offset: given by the MSI-X capability; Size: 16 bytes  $\times$  number of entries (2  $\times$  64 bits  $\times$  number of entries). See the PCI Express specification listed in section 1.2 for details of this table. The offset and number of entries are in the MSI-X capability. See section 3.7 for information on how the MSI-X table is used.

# 9.2.27 MSI-X Pending Bit Array

BAR0; Offset: given by the MSI-X capability; Size: [number of entries  $\div$  64]  $\times$  64 bits. (Note the use of the ceiling function in the above equation to round up the result of the division to the nearest integer.) See the PCI Express specification listed in section 1.2 for details of this table. The offset and number of entries are in the MSI-X capability.

# 9.2.28 Interrupt Message Storage

If the Interrupt Message Storage Size field in GENCAP is non-zero, the Interrupt Message Storage contains interrupt messages in addition to those in the MSI-X table defined in the PCI Express specification. The format of this table is like that of the MSI-X table, except that:

- The pending bit for each entry is in the Control field instead of in a separate pending bit array.

- Several additional controls are defined in the Control field.

- The size of the IMS table is not limited to 2048 entries. (However, the size of this table may vary between different Intel DSA implementations and may be less than 2048 entries.)

The offset of the Interrupt Message Storage is given by the IMS Offset field in the Table Offsets register. The number of entries is given by the Interrupt Message Storage Size field in GENCAP. See section 3.7 for information on how this table is used.

If the Interrupt Message Storage Size field in GENCAP is 0, this table is not present.

The initial value of Message Address is 00000000FEE00000h. If the value written to the Message Address field of the IMS entry does not contain 00000000FEEh in the upper 44 bits, the value written is ignored. (The previously stored value is retained.) Bits 1:0 of the value written to Message Address are ignored.

<table><tr><td colspan="5">IMS entry
Base: BARO
Offset: Table-offset + index × 16
Size: 16 bytes (4 × 32 bits)</td></tr><tr><td>Bytes</td><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>7:0</td><td>63:0</td><td>RW</td><td>64 bits</td><td>Message Address
See description for constraints on the value that may be written to this register.</td></tr><tr><td>11:8</td><td>31:0</td><td>RW</td><td>32 bits</td><td>Message Data</td></tr><tr><td rowspan="6">15:12</td><td>31:12</td><td>RW</td><td>20 bits</td><td>PASID
If PASID Enable is 1, this field is checked against the PASID field of the descriptor. See section 5.4.</td></tr><tr><td>11:4</td><td>RSVD</td><td>8 bits</td><td>Reserved.</td></tr><tr><td>3</td><td>RW</td><td>1 bit</td><td>PASID Enable
This field is checked against the WQ PASID Enable field of the WQ the descriptor was submitted to. See section 5.4.</td></tr><tr><td>2</td><td>RW</td><td>1 bit</td><td>Ignore
If this field is 1 when a descriptor completion interrupt references this IMS entry, no interrupt is generated and the Pending field is not set.
This field does not prevent delivery of an interrupt if Pending is 1 and Mask is cleared, nor does it affect delivery of interrupts due to causes other than descriptor completion.</td></tr><tr><td>1</td><td>RO</td><td>1 bit</td><td>Pending
This field is set to 1 when an interrupt is raised using this IMS entry and the Mask field is 1. This field becomes 0 when the interrupt is generated.</td></tr><tr><td>0</td><td>RW</td><td>1 bit</td><td>Mask
When this field is 1, no interrupt is generated using this IMS entry. Instead, the Pending field is set to 1. If 0 is written to this field when the Pending field is 1, an interrupt is generated.</td></tr></table>

# 9.2.29 Inter-Domain Permissions Table (IDPT)

If the Inter-Domain Support field in GENCAP is 1, the Inter-Domain Permissions Table controls the mapping of work submitters and alternate access PASIDs, corresponding permissions, and the memory regions permitted to be accessed. The offset of the Inter-Domain Permissions Table is given by the Inter-Domain Permissions Table Offset field in the Table Offsets register. The number of entries is given by the Inter-Domain Permissions Table Size field in IDCAP. See section 3.14.1 for information on how this table is used. If Inter-Domain Support in GENCAP is 0, this table is not present. The initial value of each entry in the table is 0.

The size of each IDPT entry is given by the IDPTE Size field in IDCAP. The size of each entry is  $2^{N + 5}$  bytes, where N is the value of the IDPTE Size field.

The first 32 bytes of each Inter-Domain Permissions Table entry (IDPTE) is divided into four 64-bit sub-entries. The fields of an IDPTE are read-only or read-write at different times, depending on the Usable and Allow Update fields, as detailed in the table. Any writes to fields while they are read-only are ignored. The remaining bytes of each IDPTE are reserved. See section 5.6 for a list of the checks performed on an IDPT entry when it is used.

<table><tr><td>Field</td><td>Conditions Under Which Field is Read-Only</td></tr><tr><td>Submitter Bitmap Address</td><td>Read-only while Usable is 1.</td></tr><tr><td>Window Size</td><td>Read-only while Usable is 1 or Allow Update is 1.1</td></tr><tr><td>Window Base</td><td>Read-only while Usable is 1 or Allow Update is 1.1</td></tr><tr><td>Access PASID</td><td>Read-only while Usable is 1 or Allow Update is 1.</td></tr><tr><td>Window Mode</td><td>Read-only while Usable is 1 or Allow Update is 1.1</td></tr><tr><td>Window Enable</td><td>Read-only while Usable is 1 or Allow Update is 1.1</td></tr><tr><td>Access Privilege</td><td>Read-only while Usable is 1 or Allow Update is 1.</td></tr><tr><td>Write Permissions</td><td>Read-only while Usable is 1 or Allow Update is 1.1</td></tr><tr><td>Read Permissions</td><td>Read-only while Usable is 1 or Allow Update is 1.1</td></tr><tr><td>Submitter PASID</td><td>Read-only while Usable is 1.</td></tr><tr><td>Type</td><td>Read-only while Usable is 1 or Allow Update is 1.</td></tr><tr><td>Allow Update</td><td>Read-write at all times.</td></tr><tr><td>Usable</td><td>Read-write at all times.</td></tr></table>

Table 9-10: Inter-Domain Permissions Table Entry Read-Only Status

<table><tr><td colspan="5">IDPT
Base: BARO
Offset: Table-Offset + Index × IDPTE-Size
Size: IDPTE-Size bytes</td></tr><tr><td>Bytes</td><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td rowspan="2">31:24</td><td>63:12</td><td>RWL</td><td>52 bits</td><td>Submitter Bitmap Address
This field specifies the upper 52 address bits of a 4 KB aligned submitter bitmap region in memory. Bits 11:0 are not specified and are treated as 0.
If Type is 1, a descriptor&#x27;s PASID must have a corresponding bit set to 1 in the bitmap.
If Type is 0, this field is reserved.
This field is read-only while Usable is 1.</td></tr><tr><td>11:0</td><td>RSVD</td><td>12 bits</td><td>Reserved.</td></tr><tr><td>23:16</td><td>63:0</td><td>RWL</td><td>64 bits</td><td>Window Size
Size in bytes of the memory window.
This field must be non-zero if Window Enable is 1.
This field is read-only while Usable is 1 or Allow Update is 1.
This field is reserved if Window Enable is 0.</td></tr><tr><td>15:8</td><td>63:0</td><td>RWL</td><td>64 bits</td><td>Window Base
Base address of the memory window.
This field is read-only while Usable is 1 or Allow Update is 1.
This field is reserved if Window Enable is 0.</td></tr><tr><td rowspan="4">7:4</td><td>31:12</td><td>RWL</td><td>20 bits</td><td>Access PASID
This PASID is used to access memory.
This field must match the PASID of an Update Window descriptor.
This field is read-only while Usable is 1 or Allow Update is 1.</td></tr><tr><td>11:5</td><td>RSVD</td><td>7 bits</td><td>Reserved.</td></tr><tr><td>4</td><td>RWL</td><td>1 bit</td><td>Window Mode
0: Address Mode. Address field in the descriptor must be greater than or equal to the window base and must be less than the sum of window base and window size.
1: Offset Mode. Address field in the descriptor is an offset and must be less than the window size. Descriptor address is added to the window base to compute the real address.
This field is read-only while Usable is 1 or Allow Update is 1.
This field is reserved if Window Enable is 0 or if Offset Mode Support in IDCAP is 0.</td></tr><tr><td>3</td><td>RWL</td><td>1 bit</td><td>Window Enable
0: Memory window checking is disabled, and hardware does not perform address range checks. The IDPTE grants access to the entire address space of the access PASID.
1: Memory window checking is enabled; address range check is performed against the corresponding address in the descriptor.
This field is read-only while Usable is 1 or Allow Update is 1.</td></tr></table>

<table><tr><td colspan="3">IDPT
Base: BAR0</td><td colspan="2">Offset: Table-Offset + Index × IDPTE-Size</td><td>Size: IDPTE-Size bytes</td></tr><tr><td rowspan="3"></td><td>2</td><td>RWL</td><td>1 bit</td><td colspan="2">Access Privilege
This controls the privilege used to access memory.
0: Overrides the privilege of the descriptor. Memory access using this entry specifies user privilege (priv field equal to 0).
1: Memory access using this entry uses the privilege of the descriptor.
This field is read-only while Usable is 1 or Allow Update is 1.</td></tr><tr><td>1</td><td>RWL</td><td>1 bit</td><td colspan="2">Write Permissions
0: Memory writes are not allowed using this entry.
1: Memory writes are allowed using this entry
This field is read-only while Usable is 1 or Allow Update is 1.</td></tr><tr><td>0</td><td>RWL</td><td>1 bit</td><td colspan="2">Read Permissions
0: Memory reads are not allowed using this entry.
1: Memory reads are allowed using this entry.
This field is read-only while Usable is 1 or Allow Update is 1.</td></tr><tr><td rowspan="5">3:0</td><td>31:12</td><td>RWL</td><td>20 bits</td><td colspan="2">Submitter PASID
If Type is 0, this field must match the PASID of an inter-domain descriptor.
If Type is 1, this field is reserved.
This field is read-only while Usable is 1.</td></tr><tr><td>11:4</td><td>RSVD</td><td>8 bits</td><td colspan="2">Reserved.</td></tr><tr><td>3:2</td><td>RWL</td><td>2 bits</td><td colspan="2">Type
0: Single-access, single-submitter entry (SASS)
1: Single-access, multi-submitter entry (SAMS)
2: Reserved
3: Reserved
This field is read-only while Usable is 1 or Allow Update is 1.
See section 3.14.1 for details on the different types of entries.</td></tr><tr><td>1</td><td>RW</td><td>1 bit</td><td colspan="2">Allow Update
0: Window attributes cannot be modified using an Update Window descriptor. Window attributes may be modified by writing to the table entry while Usable is 0.
1: Window attributes may be modified using an Update Window descriptor.</td></tr><tr><td>0</td><td>RW</td><td>1 bit</td><td colspan="2">Usable
0: A handle in an inter-domain descriptor may not reference this entry.
1: A handle in an inter-domain descriptor may reference this entry.</td></tr></table>

# 9.2.30 DSA Capabilities (DSACAP0)

<table><tr><td colspan="4">DSACAPO
Base: BARO
Offset: 0x180
Size: 8 bytes (64 bits)</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:55</td><td>RO</td><td>8 bits</td><td>Unused</td></tr><tr><td>55:48</td><td>RO</td><td>8 bits</td><td>Maximum Scatter Gather Descriptors in Progress
The maximum number of Scatter Gather descriptors that each engine is capable of concurrently processing at any time. This field is undefined if the Descriptors in Progress Limit Supported field is GRPCAP is 0 or if Scatter Gather operations are not supported in OPCAP.</td></tr><tr><td>47:32</td><td>RO</td><td>16 bits</td><td>SGL Formats Supported
Bitmask indicating which of the SGL formats described in section 8.1.14 are supported. Bit 0 is reserved. Bit 1 corresponds to SGL format 1, bit 2 corresponds to SGL format 2 and so on.</td></tr><tr><td>31:15</td><td>RO</td><td>17 bits</td><td>Unused</td></tr><tr><td>14:8</td><td>RO</td><td>7 bits</td><td>Operations with Inter-Domain Support
Bitmask indicating which operations support use of IDPT handles. If a bit is 0, the corresponding operation may not specify IDPT handles. If a bit is 1, the corresponding operation is allowed to specify IDPT handles.
The operation corresponding to each bit position are shown below.
Bit 8: Type Conversion
Bit 9: Reduce
Bit 10: Reduce with Dualcast
Bit 11: Unused
Bit 12: Gather Copy
Bit 13: Scatter Copy
Bit 14: Scatter Fill
Use of IDPT Handles in operations for which the corresponding bit position is 0 is not supported. This field is undefined if the Inter-Domain Support field in GENCAP is 0.</td></tr><tr><td>7:4</td><td>RO</td><td>4 bits</td><td>Maximum Supported Gather Reduce Block Size
The maximum block size in bytes supported in a Gather Reduce operation. The maximum supported size is 2N, where N is the value in this field. Both the input block size and output block size (as specified in section 8.3.22) must be less than or equal to this value.</td></tr><tr><td>3:0</td><td>RO</td><td>4 bits</td><td>Maximum Supported SGL Size
The maximum number of entries that can be specified in a Scatter Gather List may be independently specified for each WQ. This field indicates the maximum value that any WQ may be configured with. The maximum number of entries supported is 2N, where N is the value in this field.</td></tr></table>

# 9.2.31 DSA Capabilities (DSACAP1)

<table><tr><td colspan="4">DSACAP1
Base: BAR0
Offset: 0x188
Size: 8 bytes (64 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:48</td><td>RO</td><td>16 bits</td><td>Compute Operations Supported
Bitmask indicating which compute operations are supported by the device. Each bit corresponds to the compute operation type with the same code as the bit position. See section 8.1.11 for the encoding of compute operations.</td></tr><tr><td>47:32</td><td>RO</td><td>16 bits</td><td>Floating-Point Data Type Down Conversion Support
Bit 32: FP16 to FP8_E5M2
Bit 33: BF16 to FP8_E5M2
Bit 34: FP32 to FP8_E5M2
Bit 35: FP16 to FP8_E4M3
Bit 36: BF16 to FP8_E4M3
Bit 37: FP32 to FP8_E4M3
Bit 38: FP32 to FP16
Bit 39: FP32 to BF16
Bit 40: FP64 to FP32
Bits 47:41: Reserved
If a bit is 1, the corresponding conversion is supported; otherwise, it is not supported.</td></tr><tr><td>31:16</td><td>RO</td><td>16 bits</td><td>Floating-Point Data Type Up Conversion Support
Bit 16: FP8_E5M2 to FP16
Bit 17: FP8_E5M2 to BF16
Bit 18: FP8_E5M2 to FP32
Bit 19: FP8_E4M3 to FP16
Bit 20: FP8_E4M3 to BF16
Bit 21: FP8_E4M3 to FP32
Bit 22: FP16 to FP32
Bit 23: BF16 to FP32
Bit 24: FP32 to FP64
Bits 31:25: Reserved
If a bit is 1, the corresponding conversion is supported; otherwise, it is not supported.</td></tr><tr><td>15:0</td><td>RO</td><td>16 bits</td><td>Data Types Supported
Bitmask indicating which data types are supported by the device. Each bit corresponds to the data type with the same code as the bit position. See section 8.1.10 for the encoding of data types.</td></tr></table>

# 9.2.32 DSA Capabilities (DSACAP2)

<table><tr><td colspan="4">DSACAP2
Base: BAR0 Offset: 0x190 Size: 8 bytes (64 bits)</td></tr><tr><td>Bit</td><td>Attr</td><td>Size</td><td>Description</td></tr><tr><td>63:20</td><td>RO</td><td>44 bits</td><td>Unused</td></tr><tr><td>19:12</td><td>RO</td><td>8 bits</td><td>Rounding Type Support
Bits 19:16: Unused
Bit 15: Round toward Zero (RTZ)
Bit 14: Round Up (RU)
Bit 13: Round Down (RD)
Bit 12: Round to Nearest Even (RNE)
If a bit is 1, the corresponding rounding type is supported; otherwise, it is not supported.</td></tr><tr><td>11:9</td><td>RO</td><td>3 bits</td><td>Unused</td></tr><tr><td>8</td><td>RO</td><td>1 bit</td><td>Saturate Integer Result Support
0: Integer output result saturation is not supported.
1: Software can control integer output result saturation using the Saturate Integer Result flag in Compute Flags (described in section 8.1.12).</td></tr><tr><td>7</td><td>RO</td><td>1 bit</td><td>Signed Integer Support
0: Signed integer operands are not supported.
1: Software can control whether integer operands are treated as signed or unsigned values using the Treat Integers as Signed Values flag in Compute Flags (described in section 8.1.12).</td></tr><tr><td>6:4</td><td>RO</td><td>3 bits</td><td>Unused</td></tr><tr><td>3</td><td>RO</td><td>1 bit</td><td>Denormal as Zero (DAZ) Support
0: Treating a denormal operand as a zero value is not supported.
1: Software can control the handling of denormal operands using the Denormal as Zero flag in Compute Flags (described in section 8.1.12).</td></tr><tr><td>2</td><td>RO</td><td>1 bit</td><td>Flush to Zero (FTZ) Support
0: Conversion of a denormal result to a zero value is not supported.
1: Software can control the handling of a denormal result using the Flush to Zero flag in Compute Flags (described in section 8.1.12).</td></tr><tr><td>1</td><td>RO</td><td>1 bit</td><td>Unused</td></tr><tr><td>0</td><td>RO</td><td>1 bit</td><td>SourceOperand Negation Support
0: Negation of source operands is not supported.
1: Software can negate source operands using the Negate SourceOperand flags in Compute Flags (described in section 8.1.12).</td></tr></table>

# 9.3 Portals (BAR2)

Portals are used to submit descriptors to the device. Portals are located in the address space specified by BAR2. Each portal is 64 bytes in size and is located on a separate 4 KB page. This allows the portals to be independently mapped into different address spaces using CPU page tables and extended page tables.

There are four portals per WQ, as shown in Figure 9-2. When submitting work to a portal, bits 5:0 of the portal address must be 0. Bits 11:6 are ignored; thus any 64-byte-aligned address on the page can be used with the same effect.

Descriptor submissions to a portal for an SWQ must be performed using 64-byte Deferrable Memory Write transactions (DMWr). Any other write operation to an SWQ portal is ignored. Descriptor submissions to a DWQ must be performed using a 64-byte write operation. On Intel CPUs, software should use the MOVDIR64B instruction to generate a non-torn 64-byte write. A DMWr transaction to a disabled or dedicated WQ portal returns Retry. Any other write operation to a DWQ portal is ignored. Any read operation to the BAR2 address space returns 0x00 or 0xFF in all bytes. Reads to the BAR 2 address space are not ordered with respect to any other transactions to the device and cannot be used to ensure that upstream write operations have completed. See section 5.3 for more information on error checking and reporting related to portal accesses.

<table><tr><td></td><td>64-byte DMWr (ENQCMD or ENQCMDS)</td><td>64-byte posted write (MOVDIR64B)</td><td>Non-64-byte write</td><td>Read</td></tr><tr><td>Shared WQ</td><td>Supported</td><td>Ignored</td><td rowspan="3">Ignored</td><td rowspan="3">Returns 0x00 or 0xFF in all bytes</td></tr><tr><td>Dedicated WQ</td><td>Returns Retry</td><td>Supported</td></tr><tr><td>Disabled WQ</td><td>Returns Retry</td><td>Ignored</td></tr></table>

Table 9-11: Supported Portal Operations

![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/76e516dbce6ab223ac90e3305922115c57218d9f44859a2e46feca9654f243fd.jpg)



Figure 9-2: Portals


# Appendix A CRC Computation

Intel DSA computes CRC using a method that produces results matching the following description.

Intel DSA computes 32-bit CRC using the polynomial  $0 \times 1 \mathrm{ledc}6 \mathrm{f}41$  following the specification in the iSCSI Protocol (RFC 3720).

For 64-bit CRC, Intel DSA uses the polynomial 0x1ad93d23594c93659, as defined in the NVM Express specification.

The following description is adapted from RFC 3720. 'w' is the CRC Size, either 32 or 64. 'n' is the number of data bits. When the Transfer Size is not a multiple of w bits, the source data is padded on the end with zeros to a multiple of w bits.

The data bits are considered as the coefficients of a polynomial  $M(x)$  of degree  $n - 1$ . The least significant bit (bit 0) of the first byte of the data is the coefficient of the most significant term  $(x^{n - 1})$ , followed by bit 1 of the first byte, and so on through bit 7 of the highest numbered byte  $(x^0)$ .

The most significant w bits of the data are complemented.

The polynomial is multiplied by  $x^w$ , then divided by  $G(x)$ . The generator polynomial produces a remainder  $R(x)$  of degree  $\leq w - 1$ .

The coefficients of  $R(x)$  are considered a w-bit sequence.

The bit sequence is complemented, and the result is the CRC value.

The bits of the CRC value are stored in the CRC Value field of the completion record as follows: The  $x^{w-1}$  coefficient is stored in the least significant bit (bit 0) of byte 0 of the field, followed by the  $x^{w-2}$  coefficient in bit 1, and so on through the  $x^0$  coefficient in the most significant bit (bit 7) of the most-significant byte of the CRC Value field.

The CRC Seed field of the descriptor or the CRC seed read from memory follows the same byte/bit ordering described for the CRC Value field in the completion record.

Intel DSA computes CRC using the same polynomial as the CRC32 instruction described in the Intel® 64 and IA-32 Architectures Software Developer Manual. However, software that expects to use the two mechanisms interchangeably must pay attention to the CRC algorithm requirements for inversion and reflection of the CRC seed and result in order to obtain consistent results.

# Appendix B Data Integrity Field (DIF)

The Data Integrity Field (DIF) provides a system solution to protect the communication path between a host and storage device for end-to-end data integrity. Enterprise drives can be formatted with sector sizes that include an extra 8 bytes of information per sector which can be used to store integrity information. DIF was introduced as a way to use those extra bytes in an open standard.

Intel DSA performs DIF computation on a block of source data organized in 512B, 520B, 4096B, or 4104B blocks. It can check, strip, insert, or update the Guard Tag, Application Tag, and Reference Tag fields from source data and write the result to a destination buffer. The DIX generate operation may be used to generate the Guard Tag, Application Tag, and Reference Tag fields from source data, and write them to the destination address.

The 8 bytes of DIF information are divided up as follows:

- A 16-bit Guard Tag (CRC of the data, using the polynomial  $0 \times 18 \times 17$ ).

- A 16-bit Application Tag.

A 32-bit Reference Tag.

The guard tag protects the data portion of the sector. The application tag is opaque storage information. The reference tag protects against out-of-order and misdirected writes. Standardizing the contents of the protection information enables all nodes in the I/O path, including the disk itself, to verify the integrity of the data block.

![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/3258418e83601673682fca2de2e194366744dca371bfdbc464faa5b66711aa2e.jpg)


The Guard Tag, Application Tag, and Reference Tag are stored in memory with the most-significant byte at the lowest address; that is, in big-endian format.

# Reference Tag

The Reference Tag is initialized from the Reference Tag Seed field in the descriptor. The tag may be fixed or incrementing. If the tag is fixed, the seed in the descriptor is used for all blocks in the transfer. If the tag is incrementing, the seed is used for the first block in the transfer, and the value is incremented by one for each subsequent block in the transfer. If incrementing a tag value overflows the width of the tag, it wraps to 0. The final value is written to the completion record. (The final value is the value that would be used for the block after the last completed block.)

For the DIF Update operation, there are separate fields in the descriptor for the Source Reference Tag Seed and Destination Reference Tag Seed. There are separate flags to control whether the source and destination tags are fixed or incrementing. The source tag fields are used to determine the expected tag values in the source data, while the destination tag fields are used to determine the tag values to be

written to the destination. However, there is a flag in the descriptor to force the Reference Tag values read from the source to be written to the destination. In this case, the destination tag fields in the descriptor are ignored.

# Application Tag

The Application Tag is initialized from the Application Tag Seed field in the descriptor. The tag may be fixed or incrementing, and the final value is written to the completion record, similar to the Reference Tag. The Application Tag Mask is applied to the application tag value before using it. Bits in the tag value corresponding to 0 bits in the mask are retained, while bits in the tag value corresponding to 1 bits in the mask are forced to 0. If the application tag is incrementing, the mask is applied after incrementing the tag value. Depending on the bits set in the mask, the effect of incrementing the tag value may be masked off, resulting in the same tag value being used for multiple blocks. To avoid this, the application tag mask should typically be set to mask only higher-order bits. The Source Application Tag Mask may be set to OxFFFFFF to disable application tag checking.

For the DIF Update operation, there are separate fields in the descriptor to determine the source and destination Application Tag values, just as there are for the Reference Tag. There are also separate fields for the Source Application Tag Mask and Destination Application Tag Mask. As for the Reference Tag, there is a flag in the descriptor to force the Application Tag values read from the source to be written to the destination.

# Guard Tag

The Guard Tag is computed from the source data using the T10 CRC polynomial:

$$
G (x) = x ^ {1 6} + x ^ {1 5} + x ^ {1 1} + x ^ {9} + x ^ {8} + x ^ {7} + x ^ {5} + x ^ {4} + x ^ {2} + x + 1
$$

normally written as 0x18bb7.

The CRC algorithm treats the source data as a polynomial  $F(x)$ , where each bit of the source data is considered the coefficient of the corresponding term of the polynomial.  $F(x)$  is divided by  $G(x)$  to find the remainder  $R(x)$ .

$$
\frac {F (x)}{G (x)} = Q (x) + \frac {R (x)}{G (x)}
$$

The CRC is created by concatenating the coefficients of each term of the remainder  $R(x)$ .

The Guard Tag in the source data is checked using one of two methods:

1. Compute the CRC on the source data as described above and compare it to the Guard Tag in the source DIF.

2. Append the Guard Tag from the source DIF to the source data, compute the remainder, and check that the remainder is 0.

The two methods are mathematically equivalent.

The default value of the T10 CRC seed is 0. To provide flexibility to software to use a different seed, the Invert CRC Seed flag in the DIF Flags field of the descriptor causes 0xffff to be used as the CRC seed. The first 16 bits of the source data are XORed with the seed before performing the CRC computation.

The Invert CRC Result flag causes the computed CRC value to be inverted before comparing or storing the Guard Tag.

# B.1 DIF Check

The DIF Check operation is used to check the validity of the Data Integrity Fields in the source data. When performing a DIF Check operation, Intel DSA performs the following actions on each block of source data and the associated DIF:

- Optionally calculate the Guard Tag and compare it to the Guard Tag field in the source DIF value.

- Optionally verify the Application Tag and Reference Tag in the source DIF value.

- Update the Application Tag and Reference Tag for the next block of data, based on the Source DIF Flags field of the descriptor.

At least one of the Guard Tag, Application Tag, or Reference Tag should be checked; otherwise, this operation does nothing.

# B.2 DIF Insert

The DIF Insert operation is used to add Data Integrity Fields when the source data does not contain them. When performing a DIF Insert operation, the device performs the following actions on each block of source data:

Calculate the Guard Tag.

- Combine the Guard Tag, Application Tag and Reference Tag into a DIF value.

- Write the source data to the destination, appending the DIF value.

- Update the Application Tag and Reference Tag for the next block of data, based on the Destination DIF Flags field of the descriptor.

For a DIF Insert operation, the destination buffer size is given by

$$
\text {D e s t i n a t i o n B u f f e r S i z e} = T S + \left(\frac {T S}{B S}\right) \times 8
$$

where TS is the Transfer Size (source data size) and BS is the DIF Block Size.

# B.3 DIF Strip

The DIF Strip operation is used to remove Data Integrity Fields from the source data. Intel DSA can optionally check the validity of the fields as it removes them. When performing a DIF Strip operation, the device performs the following actions on each block of source data and the associated DIF:

- Optionally calculate the Guard Tag and compare it to the Guard Tag field in the source DIF value.

- Optionally verify the Application Tag and Reference Tag in the source DIF value.

- Write the source data (without the DIF) to the destination.

- Update the Application Tag and Reference Tag for the next block of data, based on the Source DIF Flags field of the descriptor.

For a DIF Strip operation, the destination buffer size is given by

$$
\text {D e s t i n a t i o n B u f f e r S i z e} = T S - \left(\frac {T S}{B S + 8}\right) \times 8
$$

where TS is the Transfer Size (source data size) and BS is the DIF Block Size.

# B.4 DIF Update

The DIF Update operation is used to replace the Data Integrity Fields in the source data with fresh values. Intel DSA can optionally check the validity of the fields in the source data. When performing a DIF

Update operation, the device performs the following actions on each block of source data and the associated DIF:

Calculate the Guard Tag value.

- Optionally compare the computed Guard Tag value to the Guard Tag field in the source DIF value.

- Optionally verify the Source Application Tag and Source Reference Tag in the source DIF value.

- Combine the computed Guard Tag, the Destination Application Tag, and the Destination Reference Tag into a destination DIF value.

- Write the source data to the destination, with the source DIF value replaced by the destination DIF value.

- Update the Source Application Tag and Source Reference Tag for the next block of data, based on the Source DIF Flags field of the descriptor.

- Update the Destination Application Tag and Destination Reference Tag for the next block of data, based on the Destination DIF Flags field of the descriptor.

For a DIF Update operation, the destination buffer size is the same as the Transfer Size.

The required destination buffer size for various DIF operations can be computed as shown in this table. If a DIF operation does not fully complete, the bytes written to the destination can be computed from the Bytes Completed field of the completion record.

BS = DIF block size

N = number of blocks to process

M = number of blocks completed

<table><tr><td></td><td>Transfer Size</td><td>Destination Buffer Size</td><td>Bytes Completed</td><td>Bytes Written to Destination (Can be computed by SW)</td></tr><tr><td>DIF-Check</td><td>(BS+8) × N</td><td>N/A</td><td>(BS+8) × M</td><td>N/A</td></tr><tr><td>DIF-Strip</td><td>(BS+8) × N</td><td>(BS) × N</td><td>(BS+8) × M</td><td>(BS) × M</td></tr><tr><td>DIF-Insert</td><td>(BS) × N</td><td>(BS+8) × N</td><td>(BS) × M</td><td>(BS+8) × M</td></tr><tr><td>DIF-Update</td><td>(BS+8) × N</td><td>(BS+8) × N</td><td>(BS+8) × M</td><td>(BS+8) × M</td></tr></table>

# B.5 DIX Generate

The DIX Insert operation is used to compute Data Integrity Fields for the specified source data. When performing a DIX Generate operation, the device performs the following actions on each block of source data:

Calculate the Guard Tag.

- Combine the Guard Tag, Application Tag, and Reference Tag into a DIF value.

- Write the DIF value to the destination.

- Update the Application Tag and Reference Tag for the next block of data, based on the Destination DIF Flags field of the descriptor.

For a DIX Generate operation, the destination buffer size is given by:

$$
\text {D e s t i n a t i o n B u f f e r S i z e} = \left(\frac {T S}{B S}\right) \times 8
$$

where TS is the Transfer Size (source data size) and BS is the DIF Block Size.

S

# Appendix C PCIe Configuration Registers

This appendix provides details of PCIe configuration registers for Intel DSA.

# Vendor ID (VID)

<table><tr><td colspan="5">VENDOR ID (VID)
Identifies the manufacturer of the device.
Base: Rootbus CFG Offset: 0x0
Default Value: 0x8086</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:0</td><td>RO</td><td>16</td><td>0x8086</td><td>Vendor ID (VID)
Indicates Intel (8086h).</td></tr></table>

# Device ID (DID)

<table><tr><td colspan="5">DEVICE ID (DID)Identifies the particular device.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x2Size: 2 bytes (16 bits)</td></tr><tr><td colspan="5">Default Value: Implementation defined</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:0</td><td>ROS</td><td>16</td><td>Implementationdefined</td><td>Device ID (DID)Allocated by the vendor.</td></tr></table>

# PCI Command (PCICMD)

<table><tr><td colspan="5">PCI COMMAND (PCICMD)The Command register provides coarse control over a device's ability to generate and respond to PCI cycles. When a 0 is written to this register, the device is logically disconnected from the PCI bus for all accesses except configuration accesses. 
Base: Rootbus CFG Offset: 0x4 
Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:11</td><td>RSVD</td><td>5</td><td>0x00</td><td>Reserved.</td></tr><tr><td>10</td><td>RO</td><td>1</td><td>0x0</td><td>Interrupt Disable (INTD) 
Controls the ability of the Function to generate INTx interrupts. This Function does not generate INTx interrupts, so this bit it hardwired to 0b.</td></tr><tr><td>9</td><td>RO</td><td>1</td><td>0x0</td><td>Fast Back-to-Back Enable (FBE) 
Does not apply to PCI Express and is hardwired to 0b.</td></tr><tr><td>8</td><td>RW</td><td>1</td><td>0x0</td><td>SERR# Enable (SEE)When Set, this bit enables reporting of Non-Fatal and Fatal errors detected by the Function to the Root Complex. Note that errors are reported if enabled either through this bit or through the PCI Express specific bits in the Device Control register.</td></tr><tr><td>7</td><td>RO</td><td>1</td><td>0x0</td><td>Wait Cycle Control (WCC)Does not apply to PCI Express and is hardwired to 0b.</td></tr><tr><td>6</td><td>RW</td><td>1</td><td>0x0</td><td>Parity Error Response Enable (PERE)This bit controls the logging of poisoned TLPs in the Master Data Parity Error bit in the Status register.</td></tr><tr><td>5</td><td>RO</td><td>1</td><td>0x0</td><td>VGA Palette Snoop Enable (VGAPSE)Does not apply to PCI Express and is hardwired to 0b.</td></tr><tr><td>4</td><td>RO</td><td>1</td><td>0x0</td><td>Memory Write and Invalidate Enable (MWIE)Does not apply to PCI Express and is hardwired to 0b.</td></tr><tr><td>3</td><td>RO</td><td>1</td><td>0x0</td><td>Special Cycle Enable (SCE)Does not apply to PCI Express and is hardwired to 0b.</td></tr><tr><td>2</td><td>RW</td><td>1</td><td>0x0</td><td>Bus Master Enable (BME)Controls the ability of the endpoint to issue Memory Read/Write requests. When set, the Function is allowed to issue Memory Requests. When clear, the Function is not allowed to issue Memory Requests. Note that as interrupt messages are in-band memory writes, setting BME to 0b disables interrupt messages as well. Requests other than Memory Requests (e.g., Completion) are not controlled by this bit.</td></tr><tr><td>1</td><td>RW</td><td>1</td><td>0x0</td><td>Memory Space Enable (MSE)Controls the Function's response to Memory Space accesses. A value of 0 disables the response. A value of 1 allows the Function to respond to Memory Space accesses.</td></tr><tr><td>0</td><td>RO</td><td>1</td><td>0x0</td><td>I/O Space Enable (IOSE) 
Controls the Function's response to I/O Space accesses. A value of 0 disables the response. 
Hardwired to 0 as this Function does not support I/O Space accesses.</td></tr></table>

# PCI Status (PCISTS)

<table><tr><td colspan="5">PCI STATUS (PCISTS)
The Status register is used to record status information for PCI bus related events.
Base: Rootbus CFG Offset: 0x6 Size: 2 bytes (16 bits)
Default Value: 0x0010</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15</td><td>RW1C</td><td>1</td><td>0x0</td><td>Detected Parity Error (DPE)
This bit is Set by a Function whenever it receives a Poisoned TLP, regardless of the state the Parity Error Response bit in the Command register.</td></tr><tr><td>14</td><td>RW1C</td><td>1</td><td>0x0</td><td>Signaled System Error (SSE)
This bit is Set when a Function sends an ERR_FATAL or ERR_NONFATAL Message, and the SERR# Enable bit in the Command register is 1.</td></tr><tr><td>13</td><td>RW1C</td><td>1</td><td>0x0</td><td>Received Master Abort (RMA)
This bit is Set when a Requester receives a Completion withUnsupported Request Completion Status.</td></tr><tr><td>12</td><td>RW1C</td><td>1</td><td>0x0</td><td>Received Target Abort (RTA)
This bit is Set when a Requester receives a Completion with Completer Abort Completion Status.</td></tr><tr><td>11</td><td>RW1C</td><td>1</td><td>0x0</td><td>Signaled Target Abort (STA)
This bit is Set when a Function completes a Posted or Non-Posted Request as a Completer Abort error.</td></tr><tr><td>10:9</td><td>RO</td><td>2</td><td>0x0</td><td>DEVSEL Timing (DT)
Does not apply to PCI Express and is hardwired to 00b.</td></tr><tr><td>8</td><td>RW1C</td><td>1</td><td>0x0</td><td>Master Data Parity Error (MDPE) 
This bit is Set by an Endpoint Function if the Parity 
Error Response bit in the Command register is 1b it 
either receives a Poisoned Completion or transmits 
a Poisoned Request.</td></tr><tr><td>7</td><td>RO</td><td>1</td><td>0x0</td><td>Fast Back-to-Back Transactions Capable (FBTC) 
Does not apply to PCI Express and is hardwired to 
0b.</td></tr><tr><td>6</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>5</td><td>RO</td><td>1</td><td>0x0</td><td>66 MHz Capable (C66) 
Does not apply to PCI Express and is hardwired to 
0b.</td></tr><tr><td>4</td><td>RO</td><td>1</td><td>0x1</td><td>Capabilities List (CL) 
Indicates the presence of an Extended Capability 
list item. Required by all PCI Express endpoints.</td></tr><tr><td>3</td><td>RO</td><td>1</td><td>0x0</td><td>Interrupt Status (INTS) 
When Set, indicates that an INTx emulation 
interrupt is pending internally in the Function. 
Hardwired to 0b as this Function does not support 
INTx.</td></tr><tr><td>2:0</td><td>RSVD</td><td>3</td><td>0x0</td><td>Reserved.</td></tr></table>

# Revision ID (RID)

<table><tr><td colspan="5">REVISION ID (RID)
This register specifies a device specific revision identifier.
Base: Rootbus CFG Offset: 0x8 Size: 1 byte (8 bits)
Default Value: 0x00</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>ROS</td><td>8</td><td>0x00</td><td>Revision ID (RID)
The value is chosen by the vendor. This field should be viewed as a vendor defined extension to the Device ID.</td></tr></table>

# Class Code Register-Level Programming Interface (CCRLPI)

<table><tr><td colspan="5">CLASS CODE REGISTER-LEVEL PROGRAMMING INTERFACE (CCRLPI)
The Class Code register is read-only and is used to identify the generic function of the device and, in some cases, a specific register-level programming interface. The lower byte identifies a specific register-level programming interface (if any) so that device independent software can interact with the device.
Base: Rootbus CFG Offset: 0x9 Size: 1 byte (8 bits)
Default Value: 0x00</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x00</td><td>Register-Level Programming Interface (RLPI)
Other system peripheral.</td></tr></table>

# Class Code Sub-Class (CCSC)

<table><tr><td colspan="5">CLASS CODE SUB-CLASS (CCSC)
The Class Code register is read-only and is used to identify the generic function of the device and, in some cases, a specific register-level programming interface. The middle byte is a sub-class code which identifies more specifically the function of the device.
Base: Rootbus     CFG Offset: 0x0A
Default Value: 0x80</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x80</td><td>Sub-Class (SC)
Other system peripheral.</td></tr></table>

# Class Code Base Class (CCBC)

<table><tr><td colspan="5">CLASS CODE BASE CLASS (CCBC)
The Class Code register is read-only and is used to identify the generic function of the device and, in some cases, a specific register-level programming interface. The upper byte is a base class code which broadly classifies the type of function the device performs.
Base: Rootbus     CFG Offset: 0x0B
Size: 1 byte (8 bits)
Default Value: 0x08</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x08</td><td>Base Class (BC)
Generic system peripheral.</td></tr></table>

# Cache Line Size (CLS)

<table><tr><td colspan="5">CACHE LINE SIZE (CLS)
The Cache Line Size register is set by the system firmware or the operating system to system cache line size.
Base: Rootbus CFG Offset: 0x0C
Size: 1 byte (8 bits)
Default Value: 0x00</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RW</td><td>8</td><td>0x00</td><td>Cache Line Size (CLS)
This field is implemented as a read-write field for legacy compatibility purposes but has no effect on any device behavior.</td></tr></table>

# Latency Timer (LATTMR)

<table><tr><td colspan="5">LATENCY TIMER (LATTMR)This register is also referred to as Primary Latency Timer for Type 1 Configuration Space headerFunctions.Base: Rootbus CFG Offset: 0x0D Size: 1 byte (8 bits)Default Value: 0x00</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x00</td><td>Latency Timer (LATTMR)The Latency Timer does not apply to PCI Express.This register is hardwired to 00h.</td></tr></table>

# Header Type (HDR)

<table><tr><td colspan="5">HEADER TYPE (HDR)This register identifies the layout of the second part of the predefined header (beginning at byte 10h in Configuration Space) and also whether or not the Device might contain multiple Functions. 
Base: Rootbus CFG Offset: 0x0E Size: 1 byte (8 bits) 
Default Value: 0x00</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7</td><td>RO</td><td>1</td><td>0x0</td><td>Multi-Function Device (MFD) 
When Clear, software must not probe for 
Functions other than Function 0. Hardwired to 0b 
as this is a single Function Device.</td></tr><tr><td>6:0</td><td>RO</td><td>7</td><td>0x00</td><td>Header Type (HT) 
Indicates Type 0 Configuration Space Header.</td></tr></table>

# Built-in Self-Test (BIST)

<table><tr><td colspan="5">BUILT-IN SELF-TEST (BIST)This optional register is used for control and status of BIST. 
Base: Rootbus     CFG Offset: 0x0F 
Size: 1 byte (8 bits) 
Default Value: 0x00</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x00</td><td>Revision ID (BIST) 
Devices that do not support BIST must always return a value of 0.</td></tr></table>

# Base Address 0 (BAR0)

<table><tr><td colspan="5">BASE ADDRESS 0 (BAR0) 
Size, type, and location of address range for control registers. 
Base: Rootbus CFG Offset: 0x10 Size: 8 bytes (64 bits) 
Default Value: 0x00000000_0000000C</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>63:4</td><td>RW</td><td>60</td><td>0x0</td><td>Address (ADDR) 
Bits N-1:4 are hardwired to 0, where 2N is the size of the BAR region.</td></tr><tr><td>3</td><td>RO</td><td>1</td><td>0x1</td><td>Pre-Fetchable (PF) 
This address map is pre-fetchable but assumes that the IP is integrated into a platform that does not do write merging beyond aligned 8-byte accesses.</td></tr><tr><td>2:1</td><td>RO</td><td>2</td><td>0x2</td><td>BAR Type (BT) 
Base register is 64 bits wide and can be mapped anywhere in the 64-bit address space.</td></tr><tr><td>0</td><td>RO</td><td>1</td><td>0x0</td><td>Memory Space Indicator (MSI) 
Base Address registers that map to Memory Space must return a 0 in bit 0.</td></tr></table>

# Base Address 2 (BAR2)

<table><tr><td colspan="5">BASE ADDRESS 2 (BAR2) 
Size, type, and location of address range for portals. 
Base: Rootbus CFG Offset: 0x18 Size: 8 bytes (64 bits) 
Default Value: 0x00000000_0000000C</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>63:4</td><td>RW</td><td>60</td><td>0x0</td><td>Address (ADDR) 
Bits N-1:4 are hardwired to 0, where 2N is the size of the BAR region.</td></tr><tr><td>3</td><td>RO</td><td>1</td><td>0x1</td><td>Pre-Fetchable (PF) 
This address map is pre-fetchable but assumes that the IP is integrated into a platform that does not do write merging beyond aligned 8-byte accesses.</td></tr><tr><td>2:1</td><td>RO</td><td>2</td><td>0x2</td><td>BAR Type (BT) 
Base register is 64 bits wide and can be mapped anywhere in the 64-bit address space.</td></tr><tr><td>0</td><td>RO</td><td>1</td><td>0x0</td><td>Memory Space Indicator (MSI) 
Base Address registers that map to Memory Space must return a 0 in bit 0.</td></tr></table>

# Sub-System Vendor ID (SSVID)

<table><tr><td colspan="5">SUB-SYSTEM VENDOR ID (SSVID)
This register (along with SSID) is used to uniquely identify the subsystem where the PCI device resides.
Base: Rootbus     CFG Offset: 0x2C
Size: 2 bytes (16 bits)
Default Value: 0x8086</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:0</td><td>RW</td><td>16</td><td>0x8086</td><td>Sub-System Vendor ID (SSVID)
This field should be written by boot SW.</td></tr></table>

# Sub-System ID (SSID)

<table><tr><td colspan="5">SUB-SYSTEM ID (SSID)
This register (along with SSVID) is used to uniquely identify the subsystem where the PCI device resides.
Base: Rootbus   CFG Offset: 0x2E          Size: 2 bytes (16 bits)
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:0</td><td>RW</td><td>16</td><td>0x0000</td><td>Sub-System ID (SSID)
This field should be written by boot SW.</td></tr></table>

# Capabilities Pointer (CAPPTR)

<table><tr><td colspan="5">CAPABILITIES POINTER (CAPPTR)This optional register is used to point to a linked list of new capabilities implemented by this device.This register is only valid if the Capabilities List bit in the Status Register is set. 
Base: Rootbus CFG Offset: 0x34 Size: 1 byte (8 bits) 
Default Value: 0x40</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x40</td><td>CAPPTR (CAPPTR)Points to PCI Express Capability.</td></tr></table>

# Interrupt Line (INTL)

<table><tr><td colspan="5">INTERRUPT LINE (INTL)
The Interrupt Line register communicates interrupt line routing information.
Base: Rootbus CFG Offset: 0x3C
Size: 1 byte (8 bits)
Default Value: 0x00</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x00</td><td>Interrupt Line (INTL)
Hardwired to 00h as this Function does not use an Interrupt Pin.</td></tr></table>

# Interrupt Pin (INTP)

<table><tr><td colspan="5">INTERRUPT PIN (INTP)The Interrupt Pin register is a read-only register that identifies the legacy interrupt Message(s) the Function uses. 
Base: Rootbus CFG Offset: 0x3D Size: 1 byte (8 bits) 
Default Value: 0x00</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x00</td><td>Interrupt Pin (INTP) 
A value of 00h indicates that the Function uses no legacy interrupt Message(s).</td></tr></table>

# Minimum Grant (MINGNT)

<table><tr><td colspan="5">MINIMUM GRANT (MINGNT)Does not apply to PCI Express.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x3E Size: 1 byte (8 bits)</td></tr><tr><td colspan="5">Default Value: 0x00</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x00</td><td>MIN_GNT (MINGNT)Hardwired to 00h.</td></tr></table>

# Maximum Latency (MAXLAT)

<table><tr><td colspan="5">MAXIMUM LATENCY (MAXLAT)Does not apply to PCI Express.</td></tr><tr><td colspan="4">Base: Rootbus CFG Offset: 0x3F</td><td>Size: 1 byte (8 bits)</td></tr><tr><td colspan="5">Default Value: 0x00</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x00</td><td>MAX_LAT (MAXLAT)Hardwired to 00h.</td></tr></table>


PCI Express Capability List (PCIECAPLST)


<table><tr><td colspan="5">PCI EXPRESS CAPABILITY LIST (PCIECAPLST) 
Enumerates the PCI Express Capability Structure in the PCI Configuration list. 
Base: Rootbus     CFG Offset: 0x40 
Size: 2 bytes (16 bits) 
Default Value: 0x8010</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:8</td><td>RO</td><td>8</td><td>0x80</td><td>Next Capability Pointer (NXTCAP) 
Offset to the next PCI Capability structure (MSI-X, in this case).</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x10</td><td>Capability ID (CAPID) 
Indicates the PCI Express Capability structure.</td></tr></table>


PCI Express Capabilities (PCIECAP)


<table><tr><td colspan="5">PCI EXPRESS CAPABILITIES (PCIECAP)Identifies PCI Express device Function type and associated capabilities.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x42 Size: 2 bytes (16 bits)Default Value: 0x0092</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:14</td><td>RSVD</td><td>2</td><td>0x0</td><td>Reserved.</td></tr><tr><td>13:9</td><td>RO</td><td>5</td><td>0x0</td><td>Interrupt Message Number (INTMSGNUM)Indicates which MSI-X vector is used for the interrupt message generated in association with any of the status bits in this Capability structure.</td></tr><tr><td>8</td><td>RO</td><td>1</td><td>0x0</td><td>Slot Implemented (SLOTIMP)No slot associated with this Function.</td></tr><tr><td>7:4</td><td>RO</td><td>4</td><td>1001b</td><td>Device Type (DEVTYPE)Indicates the specific type of this PCI Express Function. Root Complex Integrated Endpoint.</td></tr><tr><td>3:0</td><td>RO</td><td>4</td><td>0x2</td><td>Capability Version (CAPVER)Indicates the PCI-SIG defined PCI Express Capability structure version number.</td></tr></table>


Device Capabilities (DEVCAP)


<table><tr><td colspan="5">DEVICE CAPABILITIES (DEVCAP)Identifies PCI Express device Function specific capabilities.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x44 Size: 4 bytes (32 bits)</td></tr><tr><td colspan="5">Default Value: 0x10008022</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:29</td><td>RSVD</td><td>3</td><td>0x0</td><td>Reserved.</td></tr><tr><td>28</td><td>RO</td><td>1</td><td>0x1</td><td>Function Level Reset Capability (FLR)Indicates support for the Function Level reset mechanism.</td></tr><tr><td>27:16</td><td>RSVD</td><td>12</td><td>0x000</td><td>Reserved.</td></tr><tr><td>15</td><td>RO</td><td>1</td><td>0x1</td><td>Role-Based Error Reporting (RBER)Must be Set.</td></tr><tr><td>14:12</td><td>RSVD</td><td>3</td><td>0x0</td><td>Reserved.</td></tr><tr><td>11:9</td><td>RO</td><td>3</td><td>0x0</td><td>Endpoint L1 Acceptable Latency (L1LAT)Reserved.</td></tr><tr><td>8:6</td><td>RO</td><td>3</td><td>0x0</td><td>Endpoint L0s Acceptable Latency (LOSLAT)Reserved.</td></tr><tr><td>5</td><td>RO</td><td>1</td><td>0x1</td><td>Extended Tag Field Supported (ETFS)8-bit tag field supported.</td></tr><tr><td>4:3</td><td>RO</td><td>2</td><td>0x0</td><td>Phantom Functions Supported (PFS)Phantom functions are not supported.</td></tr><tr><td>2:0</td><td>RO</td><td>3</td><td>Implementationdefined</td><td>Max Payload Size Supported (MPSS)Indicates the maximum payload size that theFunction can support for TLPs.</td></tr></table>

# Device Control (DEVCTL)

<table><tr><td colspan="5">DEVICE CONTROL (DEVCTL) 
Controls PCI Express device specific parameters. 
Base: Rootbus CFG Offset: 0x48 
Size: 2 bytes (16 bits) 
Default Value: 0x2910</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15</td><td>RW</td><td>1</td><td>0x0</td><td>Initiate Function Level Reset (IFLR) 
A write of 1b initiates Function Level Reset to the Function. The value read by software from this bit is always 0b.</td></tr><tr><td>14:12</td><td>RW</td><td>3</td><td>010b</td><td>Max Read Request Size (MRRS) 
This field sets the maximum Read Request size for the Function as a Requester.</td></tr><tr><td>11</td><td>RW</td><td>1</td><td>0x1</td><td>Enable No Snoop (ENS) 
If this bit is Set, the Function is permitted to Set the No Snoop bit in the Requester Attributes of transactions it initiates that do not require hardware enforced cache coherency.</td></tr><tr><td>10:9</td><td>RSVD</td><td>2</td><td>0x0</td><td>Reserved.</td></tr><tr><td>8</td><td>RW</td><td>1</td><td>0x1</td><td>Extended Tag Field Enable (ETFE) 
This bit, in combination with the 10-Bit Tag Requester Enable bit in the Device Control 2 register, determines how many Tag field bits a Requester is permitted to use.</td></tr><tr><td>7:5</td><td>RW</td><td>3</td><td>Implementation defined</td><td>Max Payload Size (MPS) 
This field sets the maximum TLP payload size for the Function.</td></tr><tr><td>4</td><td>RW</td><td>1</td><td>0x1</td><td>Enable Relaxed Ordering (ERO) 
If this bit is Set, the Function is permitted to set the 
Relaxed Ordering bit in the Attributes field of 
transactions it initiates that do not require strong 
write ordering.</td></tr><tr><td>3</td><td>RW</td><td>1</td><td>0x0</td><td>Unsupported Request Reporting Enable (URRE) 
This bit, in conjunction with other bits, controls the 
signaling of unsupported Request Errors by 
sending error Messages.</td></tr><tr><td>2</td><td>RW</td><td>1</td><td>0x0</td><td>Fatal Error Reporting Enable (FERE) 
This bit, in conjunction with other bits, controls 
sending ERR_FATAL Messages.</td></tr><tr><td>1</td><td>RW</td><td>1</td><td>0x0</td><td>Non-Fatal Error Reporting Enable (NERE) 
This bit, in conjunction with other bits, controls 
sending ERR_NONFATAL Messages.</td></tr><tr><td>0</td><td>RW</td><td>1</td><td>0x0</td><td>Correctable Error Reporting Enable (CERE) 
This bit, in conjunction with other bits, controls 
sending ERR_COR Messages.</td></tr></table>

# Device Status (DEVSTS)

<table><tr><td colspan="5">DEVICE STATUS (DEVSTS) 
Provides information about PCI Express device (Function) specific parameters. 
Base: Rootbus CFG Offset: 0x4A 
Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:6</td><td>RSVD</td><td>10</td><td>0x000</td><td>Reserved.</td></tr><tr><td>5</td><td>RO</td><td>1</td><td>0x0</td><td>Transactions Pending (TP) 
When Set, this bit indicates that the Function has issued Non-Posted Requests that have not been completed. This bit is cleared only when all outstanding Non-Posted Requests have completed or have been terminated by the Completion Timeout mechanism. This bit will also be cleared upon the completion of an FLR.</td></tr><tr><td>4</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>3</td><td>RW1C</td><td>1</td><td>0x0</td><td>Unsupported Request Detected (URD) 
This bit indicates that the Function received an 
Unsupported Request. Errors are logged in this 
register regardless of whether error reporting is 
enabled in the Device Control register.</td></tr><tr><td>2</td><td>RW1C</td><td>1</td><td>0x0</td><td>Fatal Error Detected (FED) 
This bit indicates status of Fatal errors detected. 
Errors are logged in this register regardless of 
whether error reporting is enabled in the Device 
Control register. Errors are logged in this register 
regardless of the settings of the AER Uncorrectable 
Error Mask register.</td></tr><tr><td>1</td><td>RW1C</td><td>1</td><td>0x0</td><td>Non-fatal Error Detected (NED) 
This bit indicates status of Non-fatal errors 
detected. Errors are logged in this register 
regardless of whether error reporting is enabled in 
the Device Control register. Errors are logged in 
this register regardless of the settings of the AER 
Uncorrectable Error Mask register.</td></tr><tr><td>0</td><td>RW1C</td><td>1</td><td>0x0</td><td>Correctable Error Detected (CED) 
This bit indicates status of correctable errors 
detected. Errors are logged in this register 
regardless of whether error reporting is enabled in 
the Device Control register. Errors are logged in 
this register regardless of the settings of the AER 
Correctable Error Mask register.</td></tr></table>

# Device Capabilities 2 (DEVCAP2)

<table><tr><td colspan="5">DEVICE CAPABILITIES 2 (DEVCAP2)Identifies additional PCI Express device Function specific capabilities.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x64 Size: 4 bytes (32 bits)</td></tr><tr><td colspan="5">Default Value: 0x10730810</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>30:29</td><td>RO</td><td>2</td><td>0x0</td><td>DMWr Lengths Supported (DMWRLS)Indicates the largest supported DMWr TLP.</td></tr><tr><td>28</td><td>RO</td><td>1</td><td>0x1</td><td>DMWr Completer Supported (DMWRCS)Indicates whether this function can serve as aDMWr Completer.</td></tr><tr><td>27:24</td><td>RSVD</td><td>4</td><td>0x0</td><td>Reserved.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x64 Size: 4 bytes (32 bits)Default Value: 0x10730810</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>23:22</td><td>RO</td><td>2</td><td>0x1</td><td>Max End-End TLP Prefixes (MEETLPP)Indicates the maximum number of End-End TLP Prefixes supported by this Function.</td></tr><tr><td>21</td><td>RO</td><td>1</td><td>0x1</td><td>End-End TLP Prefix Supported (EETLPPS)Indicates Whether End-End TLP Prefix support is offered by a Function.</td></tr><tr><td>20</td><td>RO</td><td>1</td><td>0x1</td><td>Extended Fmt Field Supported (EFFS)If Set, the Function supports the 3-bit definition of the Fmt field. If Clear, the Function supports a 2-bit definition of the Fmt field.</td></tr><tr><td>19:18</td><td>RSVD</td><td>2</td><td>0x0</td><td>Reserved.</td></tr><tr><td>17</td><td>RO</td><td>1</td><td>0x1</td><td>Ten-Bit Tag Requester Supported (TBTRS)Indicates the Function supports 10-Bit Tag Requester capability.</td></tr><tr><td>16</td><td>RO</td><td>1</td><td>0x1</td><td>Ten-Bit Tag Completer Supported (TBTCS)Indicates the Function supports 10-Bit Tag Completer capability.</td></tr><tr><td>15:12</td><td>RSVD</td><td>4</td><td>0x0</td><td>Reserved.</td></tr><tr><td>11</td><td>RO</td><td>1</td><td>0x1</td><td>LTR Mechanism Supported (LTRMS)Indicates support for the Latency Tolerance Reporting (LTR) mechanism.</td></tr><tr><td>10:5</td><td>RSVD</td><td>6</td><td>0x00</td><td>Reserved.</td></tr><tr><td>4</td><td>RO</td><td>1</td><td>0x1</td><td>Completion Timeout Disable Supported (CTDS)Indicates support for the Completion Timeout Disable mechanism.</td></tr><tr><td>3:0</td><td>RO</td><td>4</td><td>Implementation defined</td><td>Completion Timeout Ranges Supported (CTRS)Indicates whether completion timeout programmability is supported.</td></tr></table>

# Device Control 2 (DEVCTL2)

<table><tr><td colspan="5">DEVICE CONTROL 2 (DEVCTL2) 
Controls additional PCI Express device specific parameters. 
Base: Rootbus CFG Offset: 0x68 Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:13</td><td>RSVD</td><td>3</td><td>0x0</td><td>Reserved.</td></tr><tr><td>12</td><td>RW</td><td>1</td><td>0x0</td><td>Ten-Bit Tag Requester Enable (TBTRE) 
When this bit is Set to 1b, the Requester is permitted to use 10-Bit tags.</td></tr><tr><td>11</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>10</td><td>RW</td><td>1</td><td>0x0</td><td>LTR Mechanism Enable (LTRME) 
When Set to 1b, this bit enables the Function to send LTR Messages.</td></tr><tr><td>9:5</td><td>RSVD</td><td>5</td><td>0x00</td><td>Reserved.</td></tr><tr><td>4</td><td>RW</td><td>1</td><td>0x0</td><td>Completion Timeout Disable (CTD) 
When Set, this bit disables the Completion Timeout mechanism.</td></tr><tr><td>3:0</td><td>Impl defined</td><td>4</td><td>0x0</td><td>Completion Timeout Value (CTV) 
Completion Timeout Value programmability is supported in some implementations.</td></tr></table>

# MSI-X Capability Header (MSIXCAPLST)

<table><tr><td colspan="5">MSI-X CAPABILITY HEADER (MSIXCAPLST) 
Enumerates the MSI-X Capability structure in the PCI Configuration Space Capability list. 
Base: Rootbus CFG Offset: 0x80 Size: 2 bytes (16 bits) 
Default Value: 0x9011</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:8</td><td>RO</td><td>8</td><td>0x90</td><td>Next Capability Pointer (NXTCAP) 
Pointer to next capability (Power Management, in this case).</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x11</td><td>Capability ID (CAPID) 
Indicates the MSI-X Capability structure.</td></tr></table>

# MSI-X Message Control (MSIXMSGCTL)

<table><tr><td colspan="5">MSI-XMESSAGE CONTROL (MSIXMSGCTL)MSI-X controls. System SW can modify bits in this register. A device driver is not permitted to modifythis register.Base: Rootbus CFG Offset: 0x82 Size: 2 bytes (16 bits)Default Value: 0x0008</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15</td><td>RW</td><td>1</td><td>0x0</td><td>MSI-X Enable (MSIXEN)If set, the Function is permitted to send MSI-X messages.</td></tr><tr><td>14</td><td>RW</td><td>1</td><td>0x0</td><td>Function Mask (FCNMSK)If set, all vectors associated with the Function are masked.</td></tr><tr><td>13:11</td><td>RSVD</td><td>3</td><td>0x0</td><td>Reserved.</td></tr><tr><td>10:0</td><td>RO</td><td>11</td><td>0x008</td><td>Table Size (TBLSZ)MSI-X Table Size. Encoded as N-1 (N = 9 entries).</td></tr></table>

# MSI-X Table (MSIXTBL)

<table><tr><td colspan="5">MSI-X TABLE (MSIXTBL)
MSI-X Table Offset and Table BIR.
Base: Rootbus CFG Offset: 0x84 Size: 4 bytes (32 bits)
Default Value: 0x00002000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:3</td><td>RO</td><td>29</td><td>0x00000400</td><td>Table Offset (OFFSET)
MSI-X Table Offset within BAR indicated by BIR.
Entire register is used, masking BIR to form a 32-bit QWORD-aligned offset.</td></tr><tr><td>2:0</td><td>RO</td><td>3</td><td>0x0</td><td>Table BIR (BIR)
Indicates the BAR used to map the MSI-X Table into Memory Space. BAR 0 at 10h.</td></tr></table>

# MSI-X Pending Bit Array (MSIXPBA)

<table><tr><td colspan="5">MSI-X PENDING BIT ARRAY (MSIXPBA) 
MSI-X PBA Offset and PBA BIR. 
Base: Rootbus     CFG Offset: 0x88 
Size: 4 bytes (32 bits) 
Default Value: 0x00003000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:3</td><td>RO</td><td>29</td><td>0x00000600</td><td>PBA Offset (OFFSET) 
MSI-X PBA Offset within BAR indicated by BIR. 
Entire register is used, masking BIR to form a 32-bit QWORD-aligned offset.</td></tr><tr><td>2:0</td><td>RO</td><td>3</td><td>0x0</td><td>PBA BIR (BIR) 
Indicates the BAR used to map the MSI-X PBA into Memory Space. BAR 0 at 10h.</td></tr></table>

# Power Management Capabilities (PMCAP)

<table><tr><td colspan="5">POWER MANAGEMENT CAPABILITIES (PMCAP)PCI Power Management Capability.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x90 Size: 4 bytes (32 bits)</td></tr><tr><td colspan="5">Default Value: 0x00030001</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:27</td><td>RSVD</td><td>5</td><td>0x00</td><td>Reserved.</td></tr><tr><td>26</td><td>RO</td><td>1</td><td>0x0</td><td>D2 Support (D2)This Function does not support the D2 Power Management State.</td></tr><tr><td>25</td><td>RO</td><td>1</td><td>0x0</td><td>D1 Support (D1)This Function does not support the D1 Power Management State.</td></tr><tr><td>24:19</td><td>RSVD</td><td>6</td><td>0x00</td><td>Reserved.</td></tr><tr><td>18:16</td><td>RO</td><td>3</td><td>011b</td><td>Version (VER)Must be hardwired to 011b per PCIe spec.</td></tr><tr><td>15:8</td><td>RO</td><td>8</td><td>0x00</td><td>Next Capability Pointer (NXTCAP)Pointer to next capability (end of list, in this case).</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x01</td><td>Capability ID (CApid)Indicates PCI Power Management Capability.</td></tr></table>

# Power Management Control/Status (PMCSR)

<table><tr><td colspan="5">POWER MANAGEMENT CONTROL/STATUS (PMCSR)This register is used to manage the PCI Function&#x27;s power management state. 
Base: Rootbus CFG Offset: 0x94 Size: 4 bytes (32 bits) 
Default Value: 0x00000008</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:4</td><td>RSVD</td><td>28</td><td>0x0000000</td><td>Reserved.</td></tr><tr><td>3</td><td>RO</td><td>1</td><td>0x1</td><td>No Soft Reset (NSR) 
This bit indicates the state of the Function after writing the Power State field to transition the Function from D3(hot) to D0.</td></tr><tr><td>2</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>1:0</td><td>RW</td><td>2</td><td>0x0</td><td>Power State (PS) 
This field is used both to determine the current power state of a Function and to set the Function into a new power state. If an unsupported, optional state value is written, the data is discarded, and no state change occurs. 00b - D0, 01b - D1 (unsupported, 10b - D2 (unsupported), 11b - D3 (hot).</td></tr></table>

# AER Extended Capability Header (AEREXTCAP)

<table><tr><td colspan="5">AER EXTENDED CAPABILITY HEADER (AEREXTCAP) 
Extended Capability Header. 
Base: Rootbus CFG Offset: 0x100 
Size: 4 bytes (32 bits) 
Default Value: 0x15020001</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:20</td><td>RO</td><td>12</td><td>0x150</td><td>Next Capability Offset (NXTCAP) 
Offset to the next PCI Express Capability structure.</td></tr><tr><td>19:16</td><td>RO</td><td>4</td><td>0x2</td><td>Capability Version (CAPVER) 
PCI-SIG defined version number indicating the 
version of the Capability structure.</td></tr><tr><td>15:0</td><td>RO</td><td>16</td><td>0x0001</td><td>Extended Capability ID (EXTCAPID) 
PCI-SIG defined ID number indicating the nature 
and format of the Extended Capability.</td></tr></table>

# Uncorrectable Error Status (ERRUNCSTS)

<table><tr><td colspan="5">UNCORRECTABLE ERROR STATUS (ERRUNCSTS) Indicates error detection status of individual errors on a PCI Express device Function. 
Base: Rootbus CFG Offset: 0x104 Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:23</td><td>RSVD</td><td>9</td><td>0x000</td><td>Reserved.</td></tr><tr><td>22</td><td>RW1CS</td><td>1</td><td>0x0</td><td>Uncorrectable Internal (UI) 
Set when an uncorrectable error not covered by AER occurs (internal parity error).</td></tr><tr><td>21</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>20</td><td>RW1CS</td><td>1</td><td>0x0</td><td>Unsupported Request (UR) 
Set when this function receives an Unsupported Request response.</td></tr><tr><td>19</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>18</td><td>RW1CS</td><td>1</td><td>0x0</td><td>Malformed TLP (MTLP) 
Set when this function receives a Malformed TLP (MPS violation).</td></tr><tr><td>17</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>16</td><td>RW1CS</td><td>1</td><td>0x0</td><td>Unexpected Completion (UC) 
Set when this function receives a completion that does not correspond to a Non-posted it issued.</td></tr><tr><td>15</td><td>RW1CS</td><td>1</td><td>0x0</td><td>Completer Abort (CA) 
Set when this function sends a completion with Completer Abort status.</td></tr><tr><td>14</td><td>RW1CS</td><td>1</td><td>0x0</td><td>Completion Timeout (CTO) 
Set when a Non-posted requested by this function is terminated via the Completion Timeout mechanism.</td></tr><tr><td>13</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>12</td><td>RW1CS</td><td>1</td><td>0x0</td><td>Poisoned TLP Received (PTLP) 
Set when a TLP received by this function is marked as poisoned.</td></tr><tr><td>11:0</td><td>RSVD</td><td>12</td><td>0x000</td><td>Reserved.</td></tr></table>

# Uncorrectable Error Mask (ERRUNCMSK)

<table><tr><td colspan="5">UNCORRECTABLE ERROR MASK (ERRUNCMSK)Controls reporting of individual errors. A masked error is not recoded or reported in the Header Logor First Error Pointer and is not reported to the PCI Express Root Complex.Base: Rootbus CFG Offset: 0x108 Size: 4 bytes (32 bits)Default Value: 0x00400000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:23</td><td>RSVD</td><td>9</td><td>0x000</td><td>Reserved.</td></tr><tr><td>22</td><td>RWS</td><td>1</td><td>0x1</td><td>Uncorrectable Internal (UI)When Set, prevents the logging and reporting of Uncorrectable Internal errors.</td></tr><tr><td>21</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>20</td><td>RWS</td><td>1</td><td>0x0</td><td>Unsupported Request (UR)When Set, prevents the logging and reporting ofUnsupported Request errors.</td></tr><tr><td>19</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>18</td><td>RWS</td><td>1</td><td>0x0</td><td>Malformed TLP (MTLP)When Set, prevents the logging and reporting ofMalformed TLP errors.</td></tr><tr><td>17</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>16</td><td>RWS</td><td>1</td><td>0x0</td><td>Unexpected Completion (UC)When Set, prevents the logging and reporting of Unexpected Completion errors.</td></tr><tr><td>15</td><td>RWS</td><td>1</td><td>0x0</td><td>Completer Abort (CA)When Set, prevents the logging and reporting ofCompleter Abort errors.</td></tr><tr><td>14</td><td>RWS</td><td>1</td><td>0x0</td><td>Completion Timeout (CTO)When Set, prevents the logging and reporting ofCompletion Timeout errors.</td></tr><tr><td>13</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>12</td><td>RWS</td><td>1</td><td>0x0</td><td>Poisoned TLP Received (PTLP)When Set, prevents the logging and reporting of Poisoned TLP errors.</td></tr><tr><td>11:0</td><td>RSVD</td><td>12</td><td>0x000</td><td>Reserved.</td></tr></table>

# Uncorrectable Error Severity (ERRUNCSEV)

<table><tr><td colspan="5">UNCORRECTABLE ERROR SEVERITY (ERRUNCSEV) 
Controls whether an individual error is reported as a Non-fatal (bit is Clear) or Fatal (bit is Set) error. 
Base: Rootbus CFG Offset: 0x10C Size: 4 bytes (32 bits) 
Default Value: 0x00440000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:23</td><td>RSVD</td><td>9</td><td>0x000</td><td>Reserved.</td></tr><tr><td>22</td><td>RWS</td><td>1</td><td>0x1</td><td>Uncorrectable Internal (UI) 
When Set, Uncorrectable Internal errors are Fatal.</td></tr><tr><td>21</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>20</td><td>RWS</td><td>1</td><td>0x0</td><td>Unsupported Request (UR) 
When Set,Unsupported Request errors are Fatal.</td></tr><tr><td>19</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>18</td><td>RWS</td><td>1</td><td>0x1</td><td>Malformed TLP (MTLP) 
When Set, Malformed TLP errors are Fatal.</td></tr><tr><td>17</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>16</td><td>RWS</td><td>1</td><td>0x0</td><td>Unexpected Completion (UC) 
When Set, Unexpected Completion errors are Fatal.</td></tr><tr><td>15</td><td>RWS</td><td>1</td><td>0x0</td><td>Completer Abort (CA) 
When Set, Completer Abort errors are Fatal.</td></tr><tr><td>14</td><td>RWS</td><td>1</td><td>0x0</td><td>Completion Timeout (CTO) 
When Set, Completion Timeout errors are Fatal.</td></tr><tr><td>13</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>12</td><td>RWS</td><td>1</td><td>0x0</td><td>Poisoned TLP Received (PTLP) 
When Set, Poisoned TLP errors are Fatal.</td></tr><tr><td>11:0</td><td>RSVD</td><td>12</td><td>0x000</td><td>Reserved.</td></tr></table>

# Correctable Error Status (ERRCORSTS)

<table><tr><td colspan="5">CORRECTABLE ERROR STATUS (ERRCORSTS) 
Reports error status of individual correctable error sources. 
Base: Rootbus CFG Offset: 0x110 Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:15</td><td>RSVD</td><td>17</td><td>0x00000</td><td>Reserved.</td></tr><tr><td>14</td><td>RW1CS</td><td>1</td><td>0x0</td><td>Corrected Internal (CI) 
Set when a Corrected Internal error is detected.</td></tr><tr><td>13</td><td>RW1CS</td><td>1</td><td>0x0</td><td>Advisory Non-Fatal (ANF) 
Set when an Error is classified as Advisory Non-fatal.</td></tr><tr><td>12:0</td><td>RSVD</td><td>13</td><td>0x0000</td><td>Reserved.</td></tr></table>

# Correctable Error Mask (ERRCORMSK)

<table><tr><td colspan="5">CORRECTABLE ERROR MASK (ERRCORMSK) 
Controls the reporting of individual correctable errors. 
Base: Rootbus     CFG Offset: 0x114 
Size: 4 bytes (32 bits) 
Default Value: 0x00002000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:15</td><td>RSVD</td><td>17</td><td>0x00000</td><td>Reserved.</td></tr><tr><td>14</td><td>RWS</td><td>1</td><td>0x1</td><td>Corrected Internal (CI) 
When Set, Corrected Internal errors are not reported.</td></tr><tr><td>13</td><td>RWS</td><td>1</td><td>0x1</td><td>Advisory Non-Fatal (ANF) 
When Set, Advisory Non-Fatal errors are not reported.</td></tr><tr><td>12:0</td><td>RSVD</td><td>13</td><td>0x0000</td><td>Reserved.</td></tr></table>

# AER Capabilities and Control (AERCAPCTL)

<table><tr><td colspan="5">AER CAPABILITIES AND CONTROL (AERCAPCTL) 
More AER information. 
Base: Rootbus     CFG Offset: 0x118 
Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:12</td><td>RSVD</td><td>20</td><td>0x00000</td><td>Reserved.</td></tr><tr><td>11</td><td>ROS</td><td>1</td><td>0x0</td><td>TLP Prefix Log Present (TLPPLP) 
If Set and the First Error Pointer is valid, indicates that the TLP Prefix Log register contains valid information.</td></tr><tr><td>10:5</td><td>RSVD</td><td>6</td><td>0x00</td><td>Reserved.</td></tr><tr><td>4:0</td><td>ROS</td><td>5</td><td>0x0</td><td>First Error Pointer (FEP) 
Identifies the bit position of the first error reported in the Uncorrectable Error Status register.</td></tr></table>

# Header Log DW1 (AERHDRLOG1)

<table><tr><td colspan="5">HEADER LOG DW1 (AERHDRLOG1)
First DWORD of the header for the TLP corresponding to a detected error.
Base: Rootbus CFG Offset: 0x11C Size: 4 bytes (32 bits)
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:0</td><td>ROS</td><td>32</td><td>0x0</td><td>Header Log (HDRLOG)
Header Log DW.</td></tr></table>

# Header Log DW2 (AERHDRLOG2)

<table><tr><td colspan="5">HEADER LOG DW2 (AERHDRLOG2)Second DWORD of the header for the TLP corresponding to a detected error. 
Base: Rootbus     CFG Offset: 0x120      Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:0</td><td>ROS</td><td>32</td><td>0x0</td><td>Header Log (HDRLOG) 
Header Log DW.</td></tr></table>

# Header Log DW3 (AERHDRLOG3)

<table><tr><td colspan="5">HEADER LOG DW3 (AERHDRLOG3)
Third DWORD of the header for the TLP corresponding to a detected error.
Base: Rootbus CFG Offset: 0x124 Size: 4 bytes (32 bits)
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:0</td><td>ROS</td><td>32</td><td>0x0</td><td>Header Log (HDRLOG)
Header Log DW.</td></tr></table>

# Header Log DW4 (AERHDRLOG4)

<table><tr><td colspan="5">HEADER LOG DW4 (AERHDRLOG4) 
Fourth DWORD of the header for the TLP corresponding to a detected error. 
Base: Rootbus     CFG Offset: 0x128          Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:0</td><td>ROS</td><td>32</td><td>0x0</td><td>Header Log (HDRLOG) 
Header Log DW.</td></tr></table>

# TLP Prefix Log Register DW1 (AERTLPPLOG1)

<table><tr><td colspan="5">TLP Prefix LOG REGISTER DW1 (AERTLPPLOG1)This register captures the End-End TLP Prefix (DW1) for the TLP corresponding to the detected error. 
Base: Rootbus CFG Offset: 0x138 Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:0</td><td>ROS</td><td>32</td><td>0x0</td><td>TLP Prefix Log (TLPPLOG) 
TLP Prefix Log DW.</td></tr></table>


TLP Prefix Log Register DW2 (AERTLPPLOG2)


<table><tr><td colspan="5">TLP Prefix LOG REGISTER DW2 (AERTLPPLOG2)This register captures the End-End TLP Prefix (DW2) for the TLP corresponding to the detected error. 
Base: Rootbus CFG Offset: 0x13C 
Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:0</td><td>ROS</td><td>32</td><td>0x0</td><td>TLP Prefix Log (TLPPLOG) 
TLP Prefix Log DW.</td></tr></table>


TLP Prefix Log Register DW3 (AERTLPPLOG3)


<table><tr><td colspan="5">TLP Prefix LOG REGISTER DW3 (AERTLPPLOG3)
This register captures the End-End TLP Prefix (DW3) for the TLP corresponding to the detected error.
Base: Rootbus CFG Offset: 0x140 Size: 4 bytes (32 bits)
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:0</td><td>ROS</td><td>32</td><td>0x0</td><td>TLP Prefix Log (TLPPLOG)
TLP Prefix Log DW.</td></tr></table>


TLP Prefix Log Register DW4 (AERTLPPLOG4)


<table><tr><td colspan="5">TLP Prefix LOG REGISTER DW4 (AERTLPPLOG4)This register captures the End-End TLP Prefix (DW4) for the TLP corresponding to the detected error. 
Base: Rootbus CFG Offset: 0x144 Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:0</td><td>ROS</td><td>32</td><td>0x0</td><td>TLP Prefix Log (TLPPLOG) 
TLP Prefix Log DW.</td></tr></table>


LTR Extended Capability Header (LTREXTCAP)


<table><tr><td colspan="5">LTR EXTENDED CAPABILITY HEADER (LTREXTCAP) 
Extended Capability Header. 
Base: Rootbus CFG Offset: 0x150 
Size: 4 bytes (32 bits) 
Default Value: 0x16010018</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:20</td><td>RO</td><td>12</td><td>0x160</td><td>Next Capability Offset (NXTCAP) 
Offset to the next PCI Express Capability structure.</td></tr><tr><td>19:16</td><td>RO</td><td>4</td><td>0x1</td><td>Capability Version (CAPVER) 
PCI-SIG defined version number indicating the 
version of the Capability structure.</td></tr><tr><td>15:0</td><td>RO</td><td>16</td><td>0x0018</td><td>Extended Capability ID (EXTCAPID) 
PCI-SIG defined ID number indicating the nature 
and format of the Extended Capability.</td></tr></table>

# Max Snoop Latency (MAXSNPLAT)

<table><tr><td colspan="5">MAX SNOOP LATENCY (MAXSNPLAT)Maximum Snoop Latency the function is permitted to request.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x154 Size: 2 bytes (16 bits)Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:13</td><td>RSVD</td><td>3</td><td>0x0</td><td>Reserved.</td></tr><tr><td>12:10</td><td>RW</td><td>3</td><td>0x0</td><td>Latency Value (SCALE)Scale of value sent in LTR message (scale = 25N ns).</td></tr><tr><td>9:0</td><td>RW</td><td>10</td><td>0x000</td><td>Latency Value (VALUE)Value sent in LTR message.</td></tr></table>

# Max No-Snoop Latency (MAXNSNPLAT)

<table><tr><td colspan="5">MAX NO-SNOOP LATENCY (MAXNSNPLAT)Maximum No-Snoop Latency the function is permitted to request. 
Base: Rootbus CFG Offset: 0x156 Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:13</td><td>RSVD</td><td>3</td><td>0x0</td><td>Reserved.</td></tr><tr><td>12:10</td><td>RW</td><td>3</td><td>0x0</td><td>Latency Value (SCALE) 
Scale of value sent in LTR message (scale = 25N ns).</td></tr><tr><td>9:0</td><td>RW</td><td>10</td><td>0x000</td><td>Latency Value (VALUE) 
Value sent in LTR message.</td></tr></table>

# TPH Extended Capability Header (TPHEXTCAP)

<table><tr><td colspan="5">TPH EXTENDED CAPABILITY HEADER (TPHEXTCAP) 
Extended Capability Header. 
Base: Rootbus CFG Offset: 0x160 
Size: 4 bytes (32 bits) 
Default Value: 0x17010017</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:20</td><td>RO</td><td>12</td><td>0x170</td><td>Next Capability Offset (NXTCAP) 
Offset to the next PCI Express Capability structure.</td></tr><tr><td>19:16</td><td>RO</td><td>4</td><td>0x1</td><td>Capability Version (CAPVER) 
PCI-SIG defined version number indicating the 
version of the Capability structure.</td></tr><tr><td>15:0</td><td>RO</td><td>16</td><td>0x0017</td><td>Extended Capability ID (EXTCApid) 
PCI-SIG defined ID number indicating the nature 
and format of the Extended Capability.</td></tr></table>

# TPH Capability (TPHCAP)

<table><tr><td colspan="5">TPH CAPABILITY (TPHCAP)TPH Capabilities.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x164 Size: 4 bytes (32 bits)</td></tr><tr><td colspan="5">Default Value: 0x00010205</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:27</td><td>RSVD</td><td>5</td><td>0x00</td><td>Reserved.</td></tr><tr><td>26:16</td><td>RO</td><td>11</td><td>0x001</td><td>ST Table Size (STTBLSIZE)Value indicates the maximum number of ST Table entries the Function may use. Software reads this field to determine the ST Table Size N, which is encoded as N-1.</td></tr><tr><td>15:11</td><td>RSVD</td><td>5</td><td>0x00</td><td>Reserved.</td></tr><tr><td>10:9</td><td>RO</td><td>2</td><td>0x1</td><td>ST Table Location (STTBLLOC)Value indicates if and where the ST Table is located.</td></tr><tr><td>8</td><td>RO</td><td>1</td><td>0x0</td><td>Extended TPH Requester Supported (EXTTPHSUPP)If set, indicates that the Function is capable of generating Requests with a TPH TLP Prefix.</td></tr><tr><td>7:3</td><td>RSVD</td><td>5</td><td>0x00</td><td>Reserved.</td></tr><tr><td>2</td><td>RO</td><td>1</td><td>0x0</td><td>Device Specific Mode Supported (DEVSPECSUPP)If set, indicates that the Function supports the Device Specific Mode of operation.</td></tr><tr><td>1</td><td>RO</td><td>1</td><td>0x0</td><td>Interrupt Vector Mode Supported (INTVECSUPP)If set, indicates that the Function supports the Interrupt Vector Modes of operation.</td></tr><tr><td>0</td><td>RO</td><td>1</td><td>0x1</td><td>No ST Mode Supported (NOSTSUPP)If set, indicates that the Function supports the No ST Mode of operation.</td></tr></table>

# TPH Requester Control Register (TPHCTL)

<table><tr><td colspan="5">TPH REQUESTER CONTROL REGISTER (TPHCTL) 
TPH Requester Capabilities. 
Base: Rootbus CFG Offset: 0x168 
Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:10</td><td>RSVD</td><td>22</td><td>0x000000</td><td>Reserved.</td></tr><tr><td>9</td><td>RO</td><td>1</td><td>0x0</td><td>TPH Requester Enable [9:9] (TPHREQEN_9_9) 
Controls the ability to issue Request TLPs using 
Extended TPH.</td></tr><tr><td>8</td><td>RW</td><td>1</td><td>0x0</td><td>TPH Requester Enable [8:8] (TPHREQEN_8_8) 
Controls the ability to issue Request TLPs using 
TPH.</td></tr><tr><td>7:3</td><td>RSVD</td><td>5</td><td>0x00</td><td>Reserved.</td></tr><tr><td>2:0</td><td>RW</td><td>3</td><td>0x0</td><td>ST Mode Select (STMODESEL) 
Selects the ST Mode of operation.</td></tr></table>

# TPHSTTable(TPHSTTBL0)

<table><tr><td colspan="5">TPH ST TABLE (TPHSTTBL0)TPH ST Table.</td></tr><tr><td colspan="4">Base: Rootbus CFG Offset: 0x16C</td><td>Size: 2 bytes (16 bits)</td></tr><tr><td colspan="5">Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:8</td><td>RO</td><td>8</td><td>0x00</td><td>ST Upper Entry (STUE)If the Function&#x27;s Extended TPH RequesterSupported bit is Set, then this field contains theupper 8 bits of a Steering Tag.</td></tr><tr><td>7:0</td><td>RW</td><td>8</td><td>0x00</td><td>ST Lower Entry (STLE)This field contains the lower 8 bits of a Steering Tag.</td></tr></table>

# TPHSTTable(TPHSTTBL1)

<table><tr><td colspan="5">TPH ST TABLE (TPHSTTBL1)TPH ST Table. 
Base: Rootbus CFG Offset: 0x16E 
Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:8</td><td>RO</td><td>8</td><td>0x00</td><td>ST Upper Entry (STUE) 
If the Function&#x27;s Extended TPH Requester Supported bit is Set, then this field contains the upper 8 bits of a Steering Tag.</td></tr><tr><td>7:0</td><td>RW</td><td>8</td><td>0x00</td><td>ST Lower Entry (STLE) 
This field contains the lower 8 bits of a Steering Tag.</td></tr></table>


VC Extended Capability Header (VCEXTCAP)


<table><tr><td colspan="5">VC EXTENDED CAPABILITY HEADER (VCEXTCAP) 
Virtual Channel Extended Capability Header. 
Base: Rootbus CFG Offset: 0x170 
Size: 4 bytes (32 bits) 
Default Value: 0x20010002</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:20</td><td>RO</td><td>12</td><td>0x200</td><td>Next Capability Offset (NXTCAP) 
Offset to the next PCI Express Capability structure.</td></tr><tr><td>19:16</td><td>RO</td><td>4</td><td>0x1</td><td>Capability Version (CAPVER) 
PCI-SIG defined version number indicating the 
version of the Capability structure.</td></tr><tr><td>15:0</td><td>RO</td><td>16</td><td>0x0002</td><td>Extended Capability ID (EXTCAPID) 
PCI-SIG defined ID number indicating the nature 
and format of the Extended Capability.</td></tr></table>


Port VC Capability Register 1 (PORTVCCAP1)


<table><tr><td colspan="5">PORT VC CAPABILITY REGISTER 1 (PORTVCCAP1)Port VC Capability Register 1.Base: Rootbus CFG Offset: 0x174Size: 4 bytes (32 bits)Default Value: 0x00000011</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:12</td><td>RSVD</td><td>20</td><td>0x00000</td><td>Reserved.</td></tr><tr><td>11:10</td><td>RO</td><td>2</td><td>0x0</td><td>Port Arbitration Table Entry Size (PATES)Indicates the size of Port Arbitration table entry in the Function. Does not apply to this Endpoint IP.</td></tr><tr><td>9:8</td><td>RO</td><td>2</td><td>0x0</td><td>Reference Clock (REFCLK)Indicates the reference clock for Virtual Channels that support time-based WRR Port Arbitration.Does not apply to this Endpoint IP.</td></tr><tr><td>7</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>6:4</td><td>RO</td><td>3</td><td>0x1</td><td>Low Priority Extended VC Count (LPEXTVCCNT)Indicates the number of (extended) Virtual Channels in addition to the default VC belonging to the low-priority VC (LPVC) group.</td></tr><tr><td>3</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>2:0</td><td>RO</td><td>3</td><td>0x1</td><td>Extended VC Count (EXTVCCNT)Indicates the number of (extended) Virtual Channels in addition to the default VC supported by the device.</td></tr></table>


Port VC Capability Register 2 (PORTVCCAP2)


<table><tr><td colspan="5">PORT VC CAPABILITY REGISTER 2 (PORTVCCAP2)Port VC Capability Register 2.Base: Rootbus CFG Offset: 0x178Size: 4 bytes (32 bits)Default Value: 0x00000001</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:24</td><td>RO</td><td>8</td><td>0x00</td><td>VC Arbitration Table Offset (VCARBTO)Indicates the location of the VC Arbitration Table.</td></tr><tr><td>23:8</td><td>RSVD</td><td>16</td><td>0x0000</td><td>Reserved.</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x01</td><td>VC Arbitration Capability (VCARBCAP)Indicates the types of VC Arbitration supported bythe Function for the LPVC.</td></tr></table>


Port VC Control Register (PORTVCCTL)


<table><tr><td colspan="5">PORT VC CONTROL REGISTER (PORTVCCTL)Port VC Control Register.</td></tr><tr><td colspan="4">Base: Rootbus CFG Offset: 0x17C</td><td>Size: 2 bytes (16 bits)</td></tr><tr><td colspan="5">Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:4</td><td>RSVD</td><td>12</td><td>0x000</td><td>Reserved.</td></tr><tr><td>3:1</td><td>RW</td><td>3</td><td>0x0</td><td>VC Arbitration Select (VCARBSEL)Used by software to configure the VC arbitration by selecting one of the supported VC Arbitration schemes indicated by the VC Arbitration schemes indicated by the VC Arbitration Capability field in the Port VC Capability register 2.</td></tr><tr><td>0</td><td>RO</td><td>1</td><td>0x0</td><td>Load VC Arbitration Table (LDVCARBTBL)Used by software to update the VC Arbitration Table. Does not apply to this IP.</td></tr></table>


Port VC Status Register (PORTVCSTS)


<table><tr><td colspan="5">PORT VC STATUS REGISTER (PORTVCSTS) 
Port VC Status Register. 
Base: Rootbus CFG Offset: 0x17E 
Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:1</td><td>RSVD</td><td>15</td><td>0x0000</td><td>Reserved.</td></tr><tr><td>0</td><td>RO</td><td>1</td><td>0x0</td><td>VC Arbitration Table Status (VCARBTBLSTS) 
Indicates the coherency status of the VC 
Arbitration Table. Does not apply to this IP.</td></tr></table>

# VC Resource Capability Register (VCOCAP)

<table><tr><td colspan="5">VC RESOURCE CAPABILITY REGISTER (VCOCAP)VC Resource Capability Register. 
Base: Rootbus CFG Offset: 0x180 
Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:24</td><td>RO</td><td>8</td><td>0x00</td><td>Port Arbitration Table Offset (PATO) 
Indicates the location of the Port Arbitration Table associated with the VC resource. Does not apply to this Endpoint IP.</td></tr><tr><td>23</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>22:16</td><td>RO</td><td>7</td><td>0x00</td><td>Maximum Time Slots (MAXTIMSLT) 
Indicates the maximum number of time slots (minus one) that the VC resource is capable of supporting when it is configured for time-based WRR Port Arbitration. Does not apply to this Endpoint IP.</td></tr><tr><td>15</td><td>RO</td><td>1</td><td>0x0</td><td>Reject Snoop Transactions (REJSNPTXN) 
When Clear, transactions with or without the No Snoop bit Set within the TLP header are allowed on this VC. Does not apply to this Endpoint IP.</td></tr><tr><td>14</td><td>RO</td><td>1</td><td>0x0</td><td>Undefined (UNDEF) 
The value read from this bit is undefined.</td></tr><tr><td>13:8</td><td>RSVD</td><td>6</td><td>0x00</td><td>Reserved.</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x00</td><td>Port Arbitration Capability (PORTARBCAP) 
Indicates types of Port Arbitration supported by the VC resource. Does not apply to this Endpoint IP.</td></tr></table>

# VC0 Resource Control Register (VCOCTL)

<table><tr><td colspan="5">VC 0 RESOURCE CONTROL REGISTER (VCOCTL) 
VC Resource Control Register. 
Base: Rootbus     CFG Offset: 0x184 
Size: 4 bytes (32 bits) 
Default Value: 0x800000FF</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31</td><td>RO</td><td>1</td><td>0x1</td><td>VC Enable (VCEN) 
This bit, when Set, enables a Virtual Channel.</td></tr><tr><td>30:27</td><td>RSVD</td><td>4</td><td>0x0</td><td>Reserved.</td></tr><tr><td>26:24</td><td>RO</td><td>3</td><td>0x0</td><td>VC ID (VCID) 
This field assigns a VC ID to the VC resource.</td></tr><tr><td>23:20</td><td>RSVD</td><td>4</td><td>0x0</td><td>Reserved.</td></tr><tr><td>19:17</td><td>RO</td><td>3</td><td>0x0</td><td>Port Arbitration Select (PORTARBSEL) 
This field configures the VC resource to provide a particular Port Arbitration service. Does not apply to this Endpoint IP.</td></tr><tr><td>16</td><td>RO</td><td>1</td><td>0x0</td><td>Load Port Arbitration Table (LDPORTARBTBL) 
When Set, this bit updates the Port Arbitration logic from the Port Arbitration Table for the VC resource. Does not apply to this Endpoint IP.</td></tr><tr><td>15:8</td><td>RSVD</td><td>8</td><td>0x00</td><td>Reserved.</td></tr><tr><td>7:1</td><td>RW</td><td>7</td><td>0x7F</td><td>TC/VC Map [7:1] (TCVCMAP_7_1) 
This field indicates the TCs that are mapped to the VC resource.</td></tr><tr><td>0</td><td>RO</td><td>1</td><td>0x1</td><td>TC/VC Map [0:0] (TCVCMAP_0_0) 
This field indicates the TCs that are mapped to the VC resource.</td></tr></table>

# VC Resource Status Register (VCOSTS)

<table><tr><td colspan="5">VC RESOURCE STATUS REGISTER (VCOSTS) 
VC Resource Status Register. 
Base: Rootbus     CFG Offset: 0x18A 
Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:2</td><td>RSVD</td><td>14</td><td>0x0000</td><td>Reserved.</td></tr><tr><td>1</td><td>RO</td><td>1</td><td>0x0</td><td>VC Negotiation Pending (VCNEGPEND) 
This bit indicates whether the Virtual Channel negotiation is in pending state. Does not apply to this non-Link IP.</td></tr><tr><td>0</td><td>RO</td><td>1</td><td>0x0</td><td>Port Arbitration Table Status (PORTARBTBLSTS) 
This bit indicates the coherency status of the Port Arbitration Table associated with the VC resource. Does not apply to this Endpoint IP.</td></tr></table>

# VC Resource Capability Register (VC1CAP)

<table><tr><td colspan="5">VC RESOURCE CAPABILITY REGISTER (VC1CAP) 
VC Resource Capability Register. 
Base: Rootbus     CFG Offset: 0x18C 
Size: 4 bytes (32 bits) 
Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:24</td><td>RO</td><td>8</td><td>0x00</td><td>Port Arbitration Table Offset (PATO) 
Indicates the location of the Port Arbitration Table associated with the VC resource. Does not apply to this Endpoint IP.</td></tr><tr><td>23</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr><tr><td>22:16</td><td>RO</td><td>7</td><td>0x00</td><td>Maximum Time Slots (MAXTIMSLT) 
Indicates the maximum number of time slots (minus one) that the VC resource is capable of supporting when it is configured for time-based WRR Port Arbitration. Does not apply to this Endpoint IP.</td></tr><tr><td>15</td><td>RO</td><td>1</td><td>0x0</td><td>Reject Snoop Transactions (REJSNPTXN) 
When Clear, transactions with or without the No Snoop bit Set within the TLP header are allowed on this VC. Does not apply to this Endpoint IP.</td></tr><tr><td>14</td><td>RO</td><td>1</td><td>0x0</td><td>Undefined (UNDEF) 
The value read from this bit is undefined.</td></tr><tr><td>13:8</td><td>RSVD</td><td>6</td><td>0x00</td><td>Reserved.</td></tr><tr><td>7:0</td><td>RO</td><td>8</td><td>0x00</td><td>Port Arbitration Capability (PORTARBCAP) 
Indicates types of Port Arbitration supported by the VC resource. Does not apply to this Endpoint IP.</td></tr></table>

# VC1 Resource Control Register (VC1CTL)

<table><tr><td colspan="5">VC 1 RESOURCE CONTROL REGISTER (VC1CTL) 
VC Resource Control Register. 
Base: Rootbus     CFG Offset: 0x190 
Size: 4 bytes (32 bits) 
Default Value: 0x01000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31</td><td>RW</td><td>1</td><td>0x0</td><td>VC Enable (VCEN) 
This bit, when Set, enables a Virtual Channel.</td></tr><tr><td>30:27</td><td>RSVD</td><td>4</td><td>0x0</td><td>Reserved.</td></tr><tr><td>26:24</td><td>RW</td><td>3</td><td>0x1</td><td>VC ID (VCID) 
This field assigns a VC ID to the VC resource.</td></tr><tr><td>23:20</td><td>RSVD</td><td>4</td><td>0x0</td><td>Reserved.</td></tr><tr><td>19:17</td><td>RO</td><td>3</td><td>0x0</td><td>Port Arbitration Select (PORTARBSEL) 
This field configures the VC resource to provide a particular Port Arbitration service. Does not apply to this Endpoint IP.</td></tr><tr><td>16</td><td>RO</td><td>1</td><td>0x0</td><td>Load Port Arbitration Table (LDPORTARBTBL) 
When Set, this bit updates the Port Arbitration logic from the Port Arbitration Table for the VC resource. Does not apply to this Endpoint IP.</td></tr><tr><td>15:8</td><td>RSVD</td><td>8</td><td>0x00</td><td>Reserved.</td></tr><tr><td>7:1</td><td>RW</td><td>7</td><td>0x00</td><td>TC/VC Map [7:1] (TCVCMAP_7_1) 
This field indicates the TCs that are mapped to the 
VC resource.</td></tr><tr><td>0</td><td>RO</td><td>1</td><td>0x0</td><td>TC/VC Map [0:0] (TCVCMAP_0_0) 
This field indicates the TCs that are mapped to the 
VC resource.</td></tr></table>

# VC Resource Status Register (VC1STS)

<table><tr><td colspan="5">VC RESOURCE STATUS REGISTER (VC1STS) 
VC Resource Status Register. 
Base: Rootbus     CFG Offset: 0x196 
Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:2</td><td>RSVD</td><td>14</td><td>0x0000</td><td>Reserved.</td></tr><tr><td>1</td><td>RO</td><td>1</td><td>0x0</td><td>VC Negotiation Pending (VCNEGPEND) 
This bit indicates whether the Virtual Channel negotiation is in pending state. Does not apply to this non-Link IP.</td></tr><tr><td>0</td><td>RO</td><td>1</td><td>0x0</td><td>Port Arbitration Table Status (PORTARBTBLSTS) 
This bit indicates the coherency status of the Port Arbitration Table associated with the VC resource. Does not apply to this Endpoint IP.</td></tr></table>

# ATS Extended Capability Header (ATSEXTCAP)

<table><tr><td colspan="5">ATS EXTENDED CAPABILITY HEADER (ATSEXTCAP) 
Extended Capability Header. 
Base: Rootbus CFG Offset: 0x220 
Size: 4 bytes (32 bits) 
Default Value: 0x2301000F</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:20</td><td>RO</td><td>12</td><td>0x230</td><td>Next Capability Offset (NXTCAP) 
Offset to the next PCI Express Capability structure.</td></tr><tr><td>19:16</td><td>RO</td><td>4</td><td>0x1</td><td>Capability Version (CAPVER) 
PCI-SIG defined version number indicating the 
version of the Capability structure.</td></tr><tr><td>15:0</td><td>RO</td><td>16</td><td>0x000F</td><td>Extended Capability ID (EXTCAPID) 
PCI-SIG defined ID number indicating the nature 
and format of the Extended Capability.</td></tr></table>

# ATS Capability (ATSCAP)

<table><tr><td colspan="5">ATS CAPABILITY (ATSCAP) 
ATS Capabilities. 
Base: Rootbus CFG Offset: 0x224 
Size: 2 bytes (16 bits) 
Default Value: 0x0060</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:7</td><td>RSVD</td><td>9</td><td>0x000</td><td>Reserved.</td></tr><tr><td>6</td><td>RO</td><td>1</td><td>0x1</td><td>Global Invalidate Supported (GIS) 
If Set, the Function supports Invalidation Requests that have the Global Invalidate bit Set.</td></tr><tr><td>5</td><td>RO</td><td>1</td><td>0x1</td><td>Page Aligned Request (PAR) 
When Set, indicates the Untranslated Address is always aligned to a 4096-byte boundary.</td></tr><tr><td>4:0</td><td>RO</td><td>5</td><td>0x00</td><td>Invalidate Queue Depth (IQD) 
Number of Invalidate Requests the Function can accept before back pressuring (00000b = 32).</td></tr></table>

# ATS Control (ATSCTL)

<table><tr><td colspan="5">ATS CONTROL (ATSCTL) 
ATS Controls. 
Base: Rootbus CFG Offset: 0x226 Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15</td><td>RW</td><td>1</td><td>0x0</td><td>Enable (EN) 
When Set, function is enabled to cache 
translations.</td></tr><tr><td>14:5</td><td>RSVD</td><td>10</td><td>0x000</td><td>Reserved.</td></tr><tr><td>4:0</td><td>RW</td><td>5</td><td>0x0</td><td>Smallest Translation Unit (STU) 
Minimum number of 4096-byte blocks that are 
indicated in a Translation Completion or Invalidate 
Request. Number of blocks = 2 ^ STU.</td></tr></table>

# PASID Extended Capability Header (PASIDEXTCAP)

<table><tr><td colspan="5">PASID EXTENDED CAPABILITY HEADER (PASIDEXTCAP) 
Extended Capability Header. 
Base: Rootbus CFG Offset: 0x230 
Size: 4 bytes (32 bits) 
Default Value: 0x2401001B</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:20</td><td>RO</td><td>12</td><td>0x240</td><td>Next Capability Offset (NXTCAP) 
Offset to the next PCI Express Capability structure.</td></tr><tr><td>19:16</td><td>RO</td><td>4</td><td>0x1</td><td>Capability Version (CAPVER) 
PCI-SIG defined version number indicating the 
version of the Capability structure.</td></tr><tr><td>15:0</td><td>RO</td><td>16</td><td>0x001B</td><td>Extended Capability ID (EXTCAPID) 
PCI-SIG defined ID number indicating the nature and format of the Extended Capability.</td></tr></table>

# PASID Capability (PASIDCAP)

<table><tr><td colspan="5">PASID CAPABILITY (PASIDCAP) PASID-related capabilities. 
Base: Rootbus CFG Offset: 0x234 Size: 2 bytes (16 bits) 
Default Value: 0x1404</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:13</td><td>RSVD</td><td>3</td><td>0x0</td><td>Reserved.</td></tr><tr><td>12:8</td><td>RO</td><td>5</td><td>0x14</td><td>Max PASID Width (MAXWID) 
PASID width supported by the function.</td></tr><tr><td>7:3</td><td>RSVD</td><td>5</td><td>0x00</td><td>Reserved.</td></tr><tr><td>2</td><td>RO</td><td>1</td><td>0x1</td><td>Privileged Mode Supported (PMS) 
If Set, function supports sending requests with the 
Privileged Mode Requested bit Set.</td></tr><tr><td>1</td><td>RO</td><td>1</td><td>0x0</td><td>Execute Permission Supported (EPS) 
If Set, function supports sending TLPs that have the 
Execute Requested bit Set.</td></tr><tr><td>0</td><td>RSVD</td><td>1</td><td>0x0</td><td>Reserved.</td></tr></table>

# PASID Control (PASIDCTL)

<table><tr><td colspan="5">PASID CONTROL (PASIDCTL) 
Controls for PASID-related functionality. 
Base: Rootbus CFG Offset: 0x236 Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:3</td><td>RSVD</td><td>13</td><td>0x0000</td><td>Reserved.</td></tr><tr><td>2</td><td>RW</td><td>1</td><td>0x0</td><td>Privileged Mode Enable (PME) 
If Set, function is permitted to send Requests with the Privileged Mode Requested bit Set.</td></tr><tr><td>1</td><td>RO</td><td>1</td><td>0x0</td><td>Execute Permission Enable (EPE) 
If Set, function is permitted to send Requests with the Execute Requested bit Set.</td></tr><tr><td>0</td><td>RW</td><td>1</td><td>0x0</td><td>PASID Enable (PE) 
Function is permitted to send and receive TLPs that contain the PASID TLP prefix.</td></tr></table>


Page Request Extended Capability Header (PRSEXTCAP)


<table><tr><td colspan="5">PAGE REQUEST EXTENDED CAPABILITY HEADER (PRSEXTCAP) 
Extended Capability Header. 
Base: Rootbus CFG Offset: 0x240 
Size: 4 bytes (32 bits) 
Default Value: 0x00010013</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:20</td><td>RO</td><td>12</td><td>0x000</td><td>Next Capability Offset (NXTCAP) 
Offset to the next PCI Express Capability structure.</td></tr><tr><td>19:16</td><td>RO</td><td>4</td><td>0x1</td><td>Capability Version (CAPVER) 
PCI-SIG defined version number indicating the 
version of the Capability structure.</td></tr><tr><td>15:0</td><td>RO</td><td>16</td><td>0x0013</td><td>Extended Capability ID (EXTCAPID) 
PCI-SIG defined ID number indicating the nature 
and format of the Extended Capability.</td></tr></table>


Page Request Control (PRSCTL)


<table><tr><td colspan="5">PAGE REQUEST CONTROL (PRSCTL) 
Controls for Page Request activities. 
Base: Rootbus CFG Offset: 0x244 
Size: 2 bytes (16 bits) 
Default Value: 0x0000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15:2</td><td>RSVD</td><td>14</td><td>0x0000</td><td>Reserved.</td></tr><tr><td>1</td><td>RW</td><td>1</td><td>0x0</td><td>Reset (RST) 
When written to 1b, clears Page Request credit counter and pending request state when Enable bit is cleared or being cleared.</td></tr><tr><td>0</td><td>RW</td><td>1</td><td>0x0</td><td>Enable (EN) 
When Set, function is allowed to make Page Requests.</td></tr></table>


Page Request Status (PRSSTS)


<table><tr><td colspan="5">PAGE REQUEST STATUS (PRSSTS)Status of Page Requests.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x246 Size: 2 bytes (16 bits)Default Value: 0x8100</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>15</td><td>RO</td><td>1</td><td>0x1</td><td>PRG Response PASID Required (PRPR)If Set, function expects a PASID on PRG ResponseMessages when corresponding Page Requests had a PASID.</td></tr><tr><td>14:9</td><td>RSVD</td><td>6</td><td>0x00</td><td>Reserved.</td></tr><tr><td>8</td><td>RO</td><td>1</td><td>0x1</td><td>Stopped (STOP)When Enable is Clear, indicates whether previously issued Page Requests have completed.</td></tr><tr><td colspan="5">PAGE REQUEST STATUS (PRSSTS)Status of Page Requests.Base: Rootbus CFG Offset: 0x246Size: 2 bytes (16 bits)Default Value: 0x8100</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>7:2</td><td>RSVD</td><td>6</td><td>0x00</td><td>Reserved.</td></tr><tr><td>1</td><td>RW1C</td><td>1</td><td>0x0</td><td>Unexpected Page Request Group Index (UPRGI)When Set, indicates the function has received a PRG Response Message containing a PRG index with no matching request.</td></tr><tr><td>0</td><td>RW1C</td><td>1</td><td>0x0</td><td>Response Failure (RF)When Set, indicates the function has received a PRG Response Message indicating a Response Failure.</td></tr></table>

# Outstanding Page Request Capacity (PRSREQCAP)

<table><tr><td colspan="5">OUTSTANDING PAGE REQUEST CAPACITY (PRSREQCAP)Maximum Number of Outstanding Page Requests.Base: Rootbus CFG Offset: 0x248 Size: 4 bytes (32 bits)Default Value: 0x00000200</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:0</td><td>RO</td><td>32</td><td>0x200</td><td>Capacity (CAP)How many Page Requests can the function issue.</td></tr></table>

# Outstanding Page Request Allocation (PRSREQALLOC)

<table><tr><td colspan="5">OUTSTANDING PAGE REQUEST ALLOCATION (PRSREQALLOC)Maximum Number of Outstanding Page Requests Allowed.</td></tr><tr><td colspan="5">Base: Rootbus CFG Offset: 0x24C Size: 4 bytes (32 bits)Default Value: 0x00000000</td></tr><tr><td>Bits</td><td>Attr</td><td>Size</td><td>Default Val</td><td>Description</td></tr><tr><td>31:0</td><td>RW</td><td>32</td><td>0x0</td><td>Enable (ALLOC)How many Page Requests will system SW allow.</td></tr></table>

S

# Appendix D Performance Monitoring Events

# D.1 Architectural Performance Monitoring Events

A set of architecturally defined performance monitoring events is common across different Intel DSA implementations. Additional events may be added in future implementations.

The Intel DSA architecture defines the following performance monitoring events.

# D.1.1 Version 1


Event Category 0: Work Queue (WQ)


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_SWQ_SUCCESS-
_LIMPORTAL</td><td>0x1</td><td>Number of successful DMWr transactions submitted to limited portal.</td><td>WQ, PASID</td></tr><tr><td>EV_SWQ_RETRY_LIMPORTAL</td><td>0x2</td><td>Number of retries returned for DMWr transactions to limited portal.</td><td>WQ, PASID</td></tr><tr><td>EV_SWQ_SUCCESS-
_UNIMPORTAL</td><td>0x4</td><td>Number of successful DMWr transactions submitted to unlimited portal.</td><td>WQ, PASID</td></tr><tr><td>EV_SWQ_RETRY-
_UNIMPORTAL</td><td>0x8</td><td>Number of retries returned for DMWr transactions to unlimited portal.</td><td>WQ, PASID</td></tr><tr><td>EV_DWQ_SUCCESS</td><td>0x10</td><td>Number of successful posted writes to DWQ.</td><td>WQ, PASID</td></tr><tr><td>EV_DWQ_FULL</td><td>0x20</td><td>Number of posted writes to DWQ dropped because queue is full.</td><td>WQ, PASID</td></tr></table>


Event Category 1: Engine


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_CL_PROCESSED</td><td>0x1</td><td>Total input data processed, in units of 32 bytes.</td><td>TC, Transfer size, Engine Number</td></tr><tr><td>EV_CL_WRITE</td><td>0x2</td><td>Total data written, in units of 32 bytes.</td><td>TC, Transfer size, Engine Number</td></tr><tr><td>EV_NUM_READ</td><td>0x4</td><td>Number of descriptors that read Source l.</td><td>TC, Transfer size, Engine Number</td></tr><tr><td>EV_NUM_WRITE</td><td>0x8</td><td>Number of descriptors that write Destination l.</td><td>TC, Transfer size, Engine Number</td></tr><tr><td>EV_NUM_DESCFROM Batch</td><td>0x10</td><td>Number of work descriptors dispatched from Batch descriptors.</td><td>WQ, Engine Number, PASID</td></tr><tr><td>EV_NUM_DESCDispatch_WQ</td><td>0x20</td><td>Number of descriptors dispatched from WQs.</td><td>WQ, Engine Number, PASID</td></tr></table>


Event Category 2: Address Translation


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_ATS_RSP_PASID_NO_PF</td><td>0x1</td><td>Number of Successful Translation completions with PASID and without page fault.</td><td>Page Size, Engine Number, PASID</td></tr><tr><td>EV_ATS_RSP_PASID_PF</td><td>0x2</td><td>Number of Successful Translation completions with PASID and with page fault.</td><td>Page size, Engine Number, PASID</td></tr><tr><td>EV_ATS_RSP_NO_PASID_NO_PF</td><td>0x4</td><td>Number of Successful Translation completions without PASID and without page fault.</td><td>Page Size, Engine Number, PASID</td></tr><tr><td>EV_ATS_RSP_NO_PASID_PF</td><td>0x8</td><td>Number of Successful Translation completions without PASID and with page fault.</td><td>Page size, Engine Number, PASID</td></tr><tr><td>EV_PRS_RSP_SUCCESS</td><td>0x10</td><td>Number of PRS Responses with Success.</td><td>PASID</td></tr><tr><td>EV_PRS_RSP_INVALID</td><td>0x20</td><td>Number of PRS Responses with Invalid Request.</td><td>PASID</td></tr></table>


Event Category 3: Operations


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_DESC_NOOP</td><td>0x1</td><td>Number of No-op descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_DESCBatch</td><td>0x2</td><td>Number of Batch descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_DESC_DRAIN</td><td>0x4</td><td>Number of Drain descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_MEM_MOVE</td><td>0x8</td><td>Number of Memory Move descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV fills</td><td>0x10</td><td>Number of Fill descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_COMPARE_MEM</td><td>0x20</td><td>Number of Compare descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_COMPARE_PAT</td><td>0x40</td><td>Number of Compare Pattern descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_CREATE_DELTA</td><td>0x80</td><td>Number of Create Delta Record descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_APPLY_DELTA</td><td>0x100</td><td>Number of Apply Delta Record descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_DIALCAST</td><td>0x200</td><td>Number of Memory Copy with Dualcast descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_CRC_GEN</td><td>0x400</td><td>Number of CRC Generation descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_COPY_CRC</td><td>0x800</td><td>Number of Copy with CRC Generation descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_DIF_CHK</td><td>0x1000</td><td>Number of DIF Check descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_DIF_INS</td><td>0x2000</td><td>Number of DIF Insert descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_DIF_STRIIP</td><td>0x4000</td><td>Number of DIF Strip descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_DIF_UPD</td><td>0x8000</td><td>Number of DIF Update descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_CLFLUSH</td><td>0x10000</td><td>Number of Cache Flush descriptors.</td><td>WQ, PASID</td></tr></table>


Event Category 4: Completions


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_NUM_MSIX</td><td>0x1</td><td>Number of MSI-X interrupts generated.</td><td>WQ, PASID</td></tr><tr><td>EV_NUM_IMS</td><td>0x2</td><td>Number of IMS interrupts generated.</td><td>WQ, PASID</td></tr><tr><td>EV_CPL_PARTIAL</td><td>0x4</td><td>Number of descriptors with partial completion.</td><td>WQ, PASID</td></tr><tr><td>EV_CPL_ERR</td><td>0x8</td><td>Number of descriptors with error completion.</td><td>WQ, PASID</td></tr><tr><td>EV_NUM_CPL_SUCC</td><td>0x10</td><td>Number of successful completions.</td><td>WQ, PASID</td></tr><tr><td>EV_NUM_CPL_WRITES</td><td>0x20</td><td>Number of completion writes.</td><td>WQ, PASID</td></tr></table>

# D.1.2 Version 2

This section lists the additions to architecturally defined performance monitoring events in implementations where the Major version field in the VERSION register (described in section 9.2.1) is 2 or greater.


Event Category 0: Work Queue (WQ)


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_WQ_DISCARD</td><td>0x40</td><td>Number of writes to a WQ that are discarded because one or more of the descriptor submission checks fail.</td><td>WQ</td></tr></table>


Event Category 1: Engine


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_NUM_IDPT_BITMAP_RD_PERM</td><td>0x40</td><td>Number of reads of IDPT bitmap where the submitter access check was successful.</td><td>Engine Number, PASID</td></tr><tr><td>EV_NUM_IDPT_BITMAP_RD_NOPERM</td><td>0x80</td><td>Number of reads of IDPT bitmap where the submitter access check was not successful.</td><td>Engine Number, PASID</td></tr><tr><td>EV_CL_READ</td><td>0x100</td><td>Total data read, in units of 32 bytes.</td><td>Engine Number</td></tr><tr><td>EV_NUM_READ2</td><td>0x200</td><td>Number of descriptors that read Source 2.</td><td>TC, Transfer size, Engine Number</td></tr><tr><td>EV_NUM_WRITE2</td><td>0x400</td><td>Number of descriptors that write Destination 2.</td><td>TC, Transfer size, Engine Number</td></tr></table>


Event Category 3: Operations


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_TRAN=FETCH</td><td>0x20000</td><td>Number of Translation Fetch descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_DIX_generate</td><td>0x40000</td><td>Number of DIX Generate descriptors.</td><td>WQ, PASID</td></tr></table>


Event Category 5: Operations 2


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_INTER_DOMAIN.copy</td><td>0x1</td><td>Number of Inter-Domain Copy descriptors.</td><td>WQ, Transfer size, PASID</td></tr><tr><td>EV_INTER_DOMAIN_fill</td><td>0x2</td><td>Number of Inter-Domain Fill descriptors.</td><td>WQ, Transfer size, PASID</td></tr><tr><td>EV_INTER_DOMAIN-_COMPARE_MEM</td><td>0x4</td><td>Number of Inter-Domain Compare descriptors.</td><td>WQ, Transfer size, PASID</td></tr></table>


Event Category 5: Operations 2


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_INTER_DOMAIN-_COMPARE_PAT</td><td>0x8</td><td>Number of Inter-Domain Compare Pattern descriptors.</td><td>WQ, Transfer size, PASID</td></tr><tr><td>EV_UPDWINDOW</td><td>0x400</td><td>Number of Update Window descriptors.</td><td>WQ, Transfer size, PASID</td></tr></table>

# D.1.3 Version 3

This section lists the additions to architecturally defined performance monitoring events in implementations where the Major version field in the VERSION register (described in section 9.2.1) is 3 or greater.


Event Category 5: Operations2


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_TYPE_CONVERT</td><td>0x800</td><td>Number of Type Conversion descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_REDIRECT</td><td>0x1000</td><td>Number of Reduce descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_REDIRECT_DUALCAST</td><td>0x2000</td><td>Number of Reduce with Dualcast descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_GATHER_REDIRECT</td><td>0x4000</td><td>Number of Gather Reduce descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_GATHER_copy</td><td>0x8000</td><td>Number of Gather Copy descriptors.</td><td>WQ, PASID</td></tr><tr><td>EV_SCATTER_copy</td><td>0x10000</td><td>Number of Scatter Copy descriptors</td><td>WQ, PASID</td></tr><tr><td>EV_SCATTER fills</td><td>0x20000</td><td>Number of Scatter Fill descriptors.</td><td>WQ, PASID</td></tr></table>

# D.2 Model-Specific Performance Monitoring Events

Model-specific performance monitoring events may be supported in addition to the architectural events defined above. These events are subject to change and may or may not be supported across different implementations of Intel DSA.

# D.2.1 Version 1

The following model-specific events are supported in implementations where the Major version field in the VERSION register (described in section 9.2.1) is 1.


Event Category 0: Work Queue (WQ)


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_CYC_NON Batch- _DESC_RDY</td><td>0x40</td><td>Number of cycles when non-batch descriptor ready.</td><td>WQ</td></tr><tr><td>EV_CYCBatch_DESC_RDY</td><td>0x80</td><td>Number of cycles when batch descriptor ready.</td><td>WQ</td></tr><tr><td>EV_CYC_DESC_NOT_RDY</td><td>0x100</td><td>Number of cycles when any of the selected WQs is empty.</td><td>WQ</td></tr></table>


Event Category 1: Engine


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_PPIPEFULL_NO_DISPATCH</td><td>0x40</td><td>Number of cycles when engine unable to dispatch descriptor to work pipeline because pipeline full.</td><td>Engine Number</td></tr><tr><td>EV_STALL_NO_DESC_RDY</td><td>0x80</td><td>Number of cycles when no descriptors ready to dispatch to work pipeline.</td><td>Engine Number</td></tr><tr><td>EV_STALL_BATCH_FETCH_FULL</td><td>0x100</td><td>Number of cycles when batch fetch-queue is full.</td><td>Engine Number</td></tr><tr><td>EV_STALL_BATCH_EXEC_FULL</td><td>0x200</td><td>Number of cycles when batch exec-queue is full.</td><td>Engine Number</td></tr></table>


Event Category 2: Address Translation


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_ATC Alloc</td><td>0x40</td><td>Number of Translation requests to ATC.</td><td>Engine Number</td></tr><tr><td>EV_ATC_NO Alloc</td><td>0x80</td><td>Number of times a translation request is unable to allocate an ATC entry.</td><td>Engine Number</td></tr><tr><td>EV_ATC_HIT_PREV</td><td>0x100</td><td>Number of times a translation request matches a valid ATC entry.</td><td>Engine Number</td></tr><tr><td>EV_CYC_INV_RSP</td><td>0x200</td><td>Number of cycles to respond to all the entries in the invalidation queue (i.e., number of cycles when invalidation queue is not empty).</td><td>None</td></tr><tr><td>EV_ATS_RSP Drops</td><td>0x400</td><td>Number of Translation Completions discarded.</td><td>Page size, Engine Number</td></tr><tr><td>EV_CYC_ATC_IDLE</td><td>0x800</td><td>Number of cycles when ATC is idle (no new requests, no outstanding ATS, etc.).</td><td>None</td></tr><tr><td>EV_INV_PASID_Q emptied</td><td>0x8000</td><td>Number of times an invalidation request with PASID is received when the invalidation queue is empty.</td><td>None</td></tr><tr><td>EV_INV_PASID_Q_NOT emptied</td><td>0x10000</td><td>Number of times an invalidation request with PASID is received when invalidation queue is not empty.</td><td>None</td></tr><tr><td>EV_INV_NO_PASID_Q emptied</td><td>0x20000</td><td>Number of times an invalidation request without PASID is received when the invalidation queue is empty.</td><td>None</td></tr><tr><td>EV_INV_NO_PASID_Q_NOT emptied</td><td>0x40000</td><td>Number of times an invalidation request without PASID is received when invalidation queue is not empty.</td><td>None</td></tr><tr><td>EV_INV_Q_FULL</td><td>0x80000</td><td>Number of times an invalidation request received caused the invalidation queue to become full.</td><td>None</td></tr></table>


Event Category 3: Operations


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_FENCE_NO Drops</td><td>0x20000</td><td>Number of fence operations not abandoned.</td><td>WQ</td></tr><tr><td>EV_FENCE Drops</td><td>0x40000</td><td>Number of fence operations abandoned.</td><td>WQ</td></tr><tr><td>EV_OVERLAP_MOV</td><td>0x80000</td><td>Number of Memory move descriptors with src-dest overlap.</td><td>WQ</td></tr></table>


Event Category 4: Completions


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_NUM_IMPLICIT-_READBACKS</td><td>0x40</td><td>Number of implicit readbacks issued.</td><td>TC</td></tr></table>

# D.2.2 Version 2

The following model-specific events are supported in implementations where the Major version field in the VERSION register (described in section 9.2.1) is 2 or 3.


Event Category 0: Work Queue (WQ)


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_CYC_NON Batch- _DESC_RDY</td><td>0x8000000</td><td>Number of cycles when non-batch descriptor ready.</td><td>WQ, PASID</td></tr><tr><td>EV_CYCBatch_DESC_RDY</td><td>0x4000000</td><td>Number of cycles when batch descriptor ready.</td><td>WQ, PASID</td></tr><tr><td>EV_CYC_DESC_NOT_RDY</td><td>0x2000000</td><td>Number of cycles when any of the selected WQs is empty.</td><td>WQ</td></tr></table>


Event Category 1: Engine


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_PPIPEFULL_NO_DISPATCH</td><td>0x8000000</td><td>Number of cycles when engine unable to dispatch descriptor to work pipeline because pipeline full.</td><td>Engine Number</td></tr><tr><td>EV_STALL_NO_DESC_RDY</td><td>0x4000000</td><td>Number of cycles when no descriptors ready to dispatch to work pipeline.</td><td>Engine Number</td></tr><tr><td>EV_STALL_BATCH-FETCH_FULL</td><td>0x2000000</td><td>Number of cycles when batch fetch-queue is full.</td><td>Engine Number</td></tr><tr><td>EV_STALL_BATCH-FETCH_FULL</td><td>0x1000000</td><td>Number of cycles when batch exec-queue is full.</td><td>Engine Number</td></tr><tr><td>EV_STALL_NOAT</td><td>0x800000</td><td>Number of cycles when engine stall due to pending Address Translation.</td><td>Engine Number</td></tr></table>


Event Category 2: Address Translation


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_ATC Alloc</td><td>0x8000000</td><td>Number of Translation requests to ATC.</td><td>Engine Number, PASID</td></tr><tr><td>EV_ATC_NO Alloc</td><td>0x4000000</td><td>Number of times a translation request is unable to allocate an ATC entry.</td><td>Engine Number, PASID</td></tr><tr><td>EV_ATC_HIT_PREV</td><td>0x2000000</td><td>Number of times a translation request matches a valid ATC entry.</td><td>Engine Number, PASID</td></tr><tr><td>EV_CYC_INV_RSP</td><td>0x1000000</td><td>Number of cycles to respond to all the entries in the invalidation queue (i.e., number of cycles when invalidation queue is not empty).</td><td>None</td></tr><tr><td>EV_ATS_RSP Drops</td><td>0x800000</td><td>Number of Translation Completions discarded.</td><td>Page size, Engine Number, PASID</td></tr><tr><td>EV_CYC_ATC_IDLE</td><td>0x400000</td><td>Number of cycles when ATC is idle (no new requests, no outstanding ATS, etc.).</td><td>None</td></tr><tr><td>EV_INV_PASID_Q emptied</td><td>0x200000</td><td>Number of times an invalidation request with PASID is received when the invalidation queue is empty.</td><td>PASID</td></tr><tr><td>EV_INV_PASID_Q_NOT emptied</td><td>0x100000</td><td>Number of times an invalidation request with PASID is received when invalidation queue is not empty.</td><td>PASID</td></tr><tr><td>EV_INV_NO_PASID_Q emptied</td><td>0x80000</td><td>Number of times an invalidation request without PASID is received when the invalidation queue is empty.</td><td>None</td></tr><tr><td>EV_INV_NO_PASID_Q_NOT emptied</td><td>0x40000</td><td>Number of times an invalidation request without PASID is received when invalidation queue is not empty.</td><td>None</td></tr><tr><td>EV_INV_Q_FULL</td><td>0x20000</td><td>Number of times an invalidation request received caused the invalidation queue to become full.</td><td>PASID</td></tr><tr><td>EV_PRS_NO Alloc</td><td>0x10000</td><td>Number of times unable to issue PRS request because of lack of credits (outstanding PRS = PRSREQALLOC).</td><td>PASID</td></tr></table>


Event Category 3: Operations


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_FENCE_NO Drops</td><td>0x8000000</td><td>Number of fence operations not abandoned.</td><td>WQ, PASID</td></tr><tr><td>EV_FENCE Drops</td><td>0x4000000</td><td>Number of fence operations abandoned.</td><td>WQ, PASID</td></tr><tr><td>EV_OVERLAP_MOV</td><td>0x2000000</td><td>Number of Memory move descriptors with src-dest overlap.</td><td>WQ, PASID</td></tr></table>


Event Category 4: Completions


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_NUM_EXPLICIT-_READBACKS</td><td>0x8000000</td><td>Number of explicit readbacks issued.</td><td>TC, PASID</td></tr><tr><td>EV_NUM_IMPLICIT-_READBACKS</td><td>0x4000000</td><td>Number of implicit readbacks issued.</td><td>TC</td></tr></table>

# D.2.3 Version 3

The following model-specific events are supported in implementations where the Major version field in the VERSION register (described in section 9.2.1) is 3.


Event Category 1: Engine


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_STALL_READ</td><td>0x400000</td><td>Number of cycles that a data read is unable to be issued because of lack of credits on the device interface.</td><td>None</td></tr><tr><td>EV_STALL_WRITE</td><td>0x200000</td><td>Number of cycles that a data write is unable to be issued because of lack of credits on the device interface.</td><td>None</td></tr><tr><td>EV_STALL_AT</td><td>0x100000</td><td>Number of cycles that an address translation request is unable to be issued because of lack of credits on the device interface.</td><td>None</td></tr><tr><td>EV_MEM_READ_LAT</td><td>0x80000</td><td>Total number of cycles elapsed between request and response for a sample set of read requests. This event should be used in conjunction with EV_MEM_READSAMPLES to identify the number of samples and compute the average.</td><td>None</td></tr><tr><td>EV_MEM_READSAMPLES</td><td>0x40000</td><td>Total number of read request samples that correspond to EV_MEM_READ_LAT.</td><td>None</td></tr></table>


Event Category 2: Address Translation


<table><tr><td>Event Name</td><td>Event Encoding</td><td>Description</td><td>Supported Filters</td></tr><tr><td>EV_TLB_REQ</td><td>0x2000000</td><td>Number of Translation requests to the Device TLB.</td><td>PASID</td></tr><tr><td>EV_TLB_HIT</td><td>0x1000000</td><td>Number of times a translation request hits in the Device TLB.</td><td>Page size, PASID</td></tr><tr><td>EV_TLB_MISS_TRK_ALLOC</td><td>0x800000</td><td>Number of times a translation request allocates into the miss tracker.</td><td>PASID</td></tr><tr><td>EV_TLB_MISS_TRK_FULL</td><td>0x400000</td><td>Number of times a translation request fails to allocate into the miss tracker because it is full.</td><td>PASID</td></tr><tr><td>EV_TLB_PENDQ_ALLOC</td><td>0x200000</td><td>Number of times a translation request allocates into the pending queue.</td><td>PASID</td></tr><tr><td>EV_TLB_PENDQ_FULL</td><td>0x100000</td><td>Number of times a translation request is unable to allocate into the pending queue because it is full.</td><td>PASID</td></tr><tr><td>EV_INV_REQ</td><td>0x80000</td><td>Number of TLB invalidation requests received.</td><td>PASID</td></tr><tr><td>EV_INV_Q_FULL</td><td>0x40000</td><td>Number of times an invalidation request received caused the invalidation queue to become full.</td><td>PASID</td></tr><tr><td>EV_INV_ATS_PEND</td><td>0x20000</td><td>Number of times an invalidation request was received that matched one or more entries in the miss tracker.</td><td>PASID</td></tr><tr><td>EV_PRS_Q_FULL</td><td>0x10000</td><td>Number of times unable to issue PRS request because of lack of credits (outstanding PRS = PRSREQALLOC).</td><td>PASID</td></tr><tr><td>EV_ATS_LAT</td><td>0x8000</td><td>Total number of cycles elapsed between request and response for a sample set of ATS requests. This event should be used in conjunction with EV_ATS_SAMPLELES to identify the number of samples and compute the average.</td><td>None</td></tr><tr><td>EV_ATS_SAMPLELES</td><td>0x4000</td><td>Total number of ATS samples that correspond to EV_ATS_LAT.</td><td>None</td></tr></table>

# D.3 Event Configuration Examples

Some event monitoring examples are shown below.

- To count the total number of attempted or successful descriptor submissions using DMWr, software can use a single counter to aggregate counts of the following events in the WQ category:

EV_SWQ_SUCCESS_LIMPORTAL - Number of successful DMWr transactions submitted to limited portal.

EV_SWQ_RETRY_LIMPORTAL - Number of retries returned for DMWr transactions to limited portal.

EV_SWQ_SUCCESS_UNLIMPORTAL - Number of successful DMWr transactions submitted to unlimited portal.

- EV_SWQ_RETRY_UNLIMPORTAL - Number of retries returned for DMWr transactions to unlimited portal.

Set CNTRCFG_0 to 0xF_00000003 (Enable=1, Interrupt on Overflow=1, Event Category=WO. Events field set to monitor the events listed above).

All filters for counter 0 set to default value of OxFFFFFF (no constraints).

- To count the number of descriptors writing memory on TC 1, from engine 1 or 2, with transfer size 4KB or higher, software can use the following event in the Engine category:

EV_NUM_WRITE - Number of writes issued.

SetFLTCFGTC1to0x2(TC1).

Set FLTCFG_SZ_1 to 0xF8 (any transfer size  $\geq$  4KB).

Set FLTCFG_ENG_1 to 0x6 (Engine 1 or 2).

Set CNTRCFG_1 to 0x8_00000103 (Enable=1, Interrupt on Overflow=1, Event Category=Engine, Events field set to monitor the event listed above).

— Other filters for counter ] set to default value of 0xFEFFF (no constraints).

- To count the number of DIF operations submitted to WQ 1 or WQ 2, software can use events in the Operations event category:

— EV DIF CHK – Number of DIF Check descriptors.

EV_DIF_INS - Number of DIF Insert descriptors.

EV_DIF Strip - Number of DIF Strip descriptors.

— EV DIF UPD – Number of DIF Update descriptors.

SetFLTCFGWO2to0x6(WO1or2).

Set CNTRCFG_2 to 0xF000_00000303 (Enable=1, Interrupt on Overflow=1, Event Category=Operations, Events field set to monitor DIF operations).

Other filters for counter 2 set to default value of 0xFFFF (no constraints).

- To estimate the frequency of occurrence of an event, software needs to use 2 distinct counters. For example, to estimate frequency (expressed as a percentage) of ATC full condition, software can program counter 0 to count EV_ATC Alloc events and counter 1 to count EV_ATC_NO Alloc events. Software then computes the ratio to estimate the frequency of occurrence of the desired condition.

S

# Appendix E Floating Point Operations

# E.1 Floating Point Data Types

Certain descriptor types specify the data type of the input and output operands. This section describes the floating-point types supported. The encodings for the different types are shown in Table 8-5 and the bit representation is shown in Figure 9-3. The sign bit is shown by the letter S, exponent is shown by the letter E, and mantissa by the letter M. The numeric range for each type is shown in Table 9-12. The representation of special numbers like infinity and NaN is shown in Table 9-13.

![image](https://cdn-mineru.openxlab.org.cn/result/2026-01-24/984d5420-d755-42c9-9247-01d24771eaff/16eafd2e5c7ae296eabc4d2337ae87701b2dff0e31e52b84072695b383874a0b.jpg)



Figure 9-3: Floating Point Data Types


<table><tr><td rowspan="2">Data Type</td><td rowspan="2">Number of Precision Bits (M+S)</td><td rowspan="2">Exponent Bias</td><td colspan="2">Approximate Normalized Range</td></tr><tr><td>Binary</td><td>Decimal</td></tr><tr><td>FP8_E5M2</td><td>3</td><td>15</td><td>2-14 to 1.75 x 215</td><td>6.10 x 10-5to 5.7344 x 104</td></tr><tr><td>FP8_E4M3</td><td>4</td><td>7</td><td>2-6 to 1.75 x 28</td><td>1.56 x 10-2to 4.48 x 102</td></tr><tr><td>FP16</td><td>11</td><td>15</td><td>2-14 to 216</td><td>6.10 x 10-5to 6.5535 x 104</td></tr><tr><td>BF16</td><td>8</td><td>127</td><td>2-126 to 2128</td><td>1.18 x 10-38to 3.40 x 1038</td></tr><tr><td>FP32</td><td>24</td><td>127</td><td>2-126 to 2128</td><td>1.18 x 10-38to 3.40 x 1038</td></tr><tr><td>FP64</td><td>53</td><td>1023</td><td>2-1022 to 21024</td><td>2.23 x 10-308to 1.80 x 10308</td></tr></table>

Table 9-12: Numeric Range for Floating Point Types

<table><tr><td>Data Type</td><td>Class</td><td>Sign</td><td>Biased Exponent</td><td>Mantissa</td></tr><tr><td rowspan="2">FP8_E5M2</td><td>NaN</td><td>S</td><td>11111</td><td>01,10,11</td></tr><tr><td>Infinity</td><td>S</td><td>11111</td><td>00</td></tr><tr><td rowspan="2">FP8_E4M3</td><td>NaN</td><td>S</td><td>1111</td><td>111</td></tr><tr><td>Infinity</td><td>S</td><td>NA</td><td>NA</td></tr><tr><td rowspan="2">FP16</td><td>NaN</td><td>S</td><td>11...11</td><td>1X...XX</td></tr><tr><td>Infinity</td><td>S</td><td>11...11</td><td>00...00</td></tr><tr><td rowspan="2">BF16</td><td>NaN</td><td>S</td><td>11...11</td><td>1X...XX</td></tr><tr><td>Infinity</td><td>S</td><td>11...11</td><td>00...00</td></tr><tr><td rowspan="2">FP32</td><td>NaN</td><td>S</td><td>11...11</td><td>1X...XX</td></tr><tr><td>Infinity</td><td>S</td><td>11...11</td><td>00...00</td></tr><tr><td rowspan="2">FP64</td><td>NaN</td><td>S</td><td>11...11</td><td>1X...XX</td></tr><tr><td>Infinity</td><td>S</td><td>11...11</td><td>00...00</td></tr></table>

Table 9-13: NaN and Infinity for Floating Point Types

# E.2 Compatibility with CPU

This section describes some similarities and differences between compute operations on Intel DSA operating on floating-point data types compared to their counterparts on the CPU.

- The representation of floating point numbers, including special numbers like infinity and NaN, match those defined in the Intel® 64 and IA-32 Architectures Software Developer's Manual (Volume 1).

- Some AVX512 CPU instructions operating on FP16 input type (for example, VCVTPH2PS) ignore the DAZ control and handle denormal values as if DAZ=0. Intel DSA does not ignore the DAZ control for FP16 inputs. In order to get behavior similar to the CPU instructions, software using Intel DSA can set the DAZ flag in Compute Flags to 0.

- Some AVX512 CPU instructions producing an FP16 result (for example, VCVTPS2PH) ignore the FTZ control and write a denormal result in case of an underflow. Intel DSA does not ignore the FTZ control with FP16 output type. In order to get behavior similar to the CPU instructions, software using Intel DSA can set the FTZ flag in Compute Flags to 0.

- Intel DSA doesn't distinguish a Signaling NaN from a Quiet NaN as an input to an operation; both types of NaNs are treated the same. It does not halt execution of an operation due to an SNaN or other floating point exception. For an operation involving a NaN where the CPU would signal a floating-point exception, Intel DSA generates a QNaN as the numeric result and sets the Invalid Operation flag in the Result field of the completion record.

- When both operands to an operation are NaNs, the result generated by Intel DSA is the first source operand, which matches the behavior of CPU floating point instructions.

# Appendix F Summary of Key Architecture Extensions

This section lists the key architecture extensions introduced in each revision of the Intel DSA specification. Software should consult the capability registers in the device (as described in section 9.2) to identify the presence of specific features in an implementation.


New features in revision 2.0


<table><tr><td>Inter-Domain Operations</td><td>New set of Inter-Domain operations that can operate on multiple address spaces (identified by PASID) with a single descriptor.
New Inter-Domain Permissions Table to facilitate connections between different address spaces.
Use with host OS/VMM, guest OS, and host or guest applications. 
(Refer to section 3.14 for details.)</td></tr><tr><td>64-bit CRC</td><td>Extend CRC operations to support 64-bit CRC (Rocksoft polynomial); Intel DSA 1.0 limited to CRC16/32. (Refer to section 8.3.12 and Appendix A.)</td></tr><tr><td>16-byte Fill</td><td>Extend Fill operation to support larger 16-byte pattern size; Intel DSA 1.0 limited to 8-byte pattern size. (Section 8.3.5.)</td></tr><tr><td>Event Log</td><td>Support for a software-configurable event log in memory to report various types of software error events. (Section 5.9.)</td></tr><tr><td>Changes in completion record</td><td>Additional field in completion record to identify operand causing page fault or other software error. (Section 8.2.3.)</td></tr><tr><td>Performance Monitoring Changes</td><td>Support for new event category and filter type, and additional performance monitoring events. (Chapter 6.)</td></tr><tr><td>Translation Fetch Descriptor</td><td>Addition of a new Translation Fetch descriptor to allow software to prefetch address translations into the device and warm up system IOTLBs to reduce address translation latency. (Section 8.3.11.)</td></tr><tr><td>WQ OPCFG Support</td><td>Ability to control which operations are supported at a work queue granularity. (Section 9.2.24.)</td></tr><tr><td>Engine Pipeline Depth Control</td><td>Ability to discover and control number of outstanding work descriptors and batch descriptors in each engine. (Section 4.4.)</td></tr><tr><td>DIX Operation</td><td>New DIX Generate operation. Similar to existing DIF Insert operation, but with the DIF field written to a distinct buffer instead of being interspersed with the source data blocks. (Section 8.3.18.)</td></tr></table>


New features in revision 3.0


<table><tr><td>Numeric Operations</td><td>New operations Type Conversion, Reduce, Reduce with Dualcast, and Gather Reduce. (Refer to sections 8.3.19 through 8.3.22.)</td></tr><tr><td>Scatter/Gather Operations</td><td>New operations Gather Copy, Scatter Copy, and Scatter Fill. (Sections 8.3.23, 8.3.24, and 8.3.25.)</td></tr><tr><td>Bandwidth Control</td><td>New controls for software to limit the maximum read and write bandwidth per group, expressed as a fraction of the maximum bandwidth supported by the device implementation. (Section 4.6.)</td></tr><tr><td>Single-entry Batch</td><td>A Batch operation may contain only a single entry. (Section 3.8.)</td></tr><tr><td>Cache Control Flags</td><td>Additional cache control flag in descriptor to allow more precise control over directing writes to cache and durable memory and performing destination readback. (Sections 3.9, 3.10, and 3.11.)</td></tr></table>