# Driver review, 2026-10-25

## Outcome

WinDivert remains the intended native packet backend, but its binary dependency
is not approved. The read-only elevated helper check passed; it does not establish
that a driver can load or that forwarding works. No new PS3 test is needed yet.

The latest release returned by the upstream API is v2.2.2 (2022-09-21), with one
405137-byte WinDivert-2.2.2-A.zip asset and no supplied digest. Upstream documents
Windows 10/11 support and says its prebuilt drivers are signed. The library opens
and installs the driver as needed; manually installing it is not a missing setup
step. These claims do not override this machine's security result.

Sources: https://github.com/basil00/WinDivert/releases/tag/v2.2.2 and
https://reqrypt.org/windivert-doc.html . The raw metadata observation is retained
in driver-review-observation-20261025.json.

Local Defender detections at 22:19:01 and 22:19:09 China time identify the project's
WinDivert ZIP. Threat 451610 is Misleading:Win32/Malgent!AMTB; action succeeded,
inactive status and no execution were reported. These selected records identify
the archive, not p2-middleware.exe; they do not exclude unrelated detections.
The archive's contents, hash and triggering member remain unverified.

An upstream report describes this same archive/detection. At review time it is
open with one comment from another reporter, without a maintainer resolution or
Microsoft determination. Its published hashes belong to that reporter's files;
we did not import them as approved identities for our unreadable local artifact.
https://github.com/basil00/WinDivert/issues/411

## App inspection

Version 0.1.4 inspects only the expected WinDivert.dll and WinDivert64.sys in the
app's drivers folder. It never opens the blocked runtime archive, downloads or
extracts dependencies. Readability, bounded x64 PE headers, SHA256 and Windows
Authenticode status are reported separately. Valid signatures do not automatically
approve deployment. No inspected identity changes the compiled approval gate.

The file stays open denying writes/deletion while its hash and signature are
checked. Files over 16 MiB, invalid/truncated headers, other architectures and
final-path reparse points are rejected. This is a bounded header check, not full
PE validation or proof of harmlessness. External signature descriptions are not
echoed into the UI. Inspection is asynchronous, with existing command timeouts.

Tests use inert bytes and our own test executable. No unreviewed DLL was loaded,
no driver installed, and no Windows protection, routing or firewall setting changed.

## Next dependency decision

A publisher/Microsoft artifact-specific review or a normally accessible reviewed
replacement is still needed. The legitimate submission channel is Microsoft's
file-analysis portal: https://www.microsoft.com/en-us/wdsi/filesubmission . No
sample was restored/uploaded and no maintainer message was sent in this task.
The existing upstream issue already records the exact detection, so another
identical report is not a technical resolution.

Once normal access is possible, independently verify source provenance, archive
and member hashes, x64 artifacts, driver signature and license obligations before
configuring approved identities. Then test filters, local socket injection,
forwarding, stop/crash recovery and ordinary PSN traffic before another co-op RUN.

Existing externally routed bridge mode remains available for controlled research.
Native Windows tester distribution remains pending. A different packet backend
would need its own implementation and driver/deployment review; no replacement
has been selected or installed in this increment.

## 2026-10-26 follow-up

Official latest-release page still resolves to v2.2.2 and issue411 remains open.
Selected local project Defender records still identify only the blocked archive.
No restoration, redownload, exclusion or driver load was performed. Idle app
startup/close now passes from the local installation; it is independent of this
packet dependency. A direct IP-alias alternative was considered but not selected:
Microsoft documents that New-NetIPAddress disables DHCP on the target interface,
so adding aliases to arbitrary users' LAN adapters is not a drop-in replacement.
No alias, DHCP or host-route experiment was performed.
Reference: https://learn.microsoft.com/en-us/powershell/module/nettcpip/new-netipaddress
