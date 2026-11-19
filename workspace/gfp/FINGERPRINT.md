# Fingerprint reader — Goodix on USB (`27c6:5e0a`)

Status (2026-11-19): the **structural** part of the reader exists and is
host-tested — crate `xaqu-goodix-fp`
(`src/lib/drivers/usb/goodix-fp/`), 20 host tests, clean under `cargo
clippy --all-targets` and `cargo fmt --check`. The **vendor** part does
not exist and is not guessed: the opcodes that ask the sensor to capture,
enroll, match, store and load are not published, so the crate carries an
empty table of them and refuses every request built from it. **No claim of
working enrollment is made anywhere in this tree.**

## 1. The device

| what | value | source |
| --- | --- | --- |
| Sensor | Goodix fingerprint reader | device enumeration on the target laptop |
| Bus | USB | device enumeration on the target laptop |
| Vendor id | `0x27c6` | device enumeration on the target laptop |
| Product id | `0x5e0a` | device enumeration on the target laptop |
| Class | HID (a fingerprint reader with an interrupt endpoint) | device enumeration on the target laptop |
| Vendor protocol | **not published** | — |
| Image geometry | **not published** | not in the HID report descriptor either |

The sensor is a *different* device from the Goodix touchpad of
`docs/I2C-TOUCHPAD.md` (GXTP7863, on I2C0 at `0x5D`): that one is a
HID-over-I2C digitizer, this one is a fingerprint reader on USB. They
share a vendor and nothing else, and this document is about the USB one.

## 2. What is implemented

All of it is pure logic over bytes — no bus, no USB transfer, no timing —
so the host tests can reach every line of it with hand-written fixtures.
The HID report descriptor itself is parsed by the repository's own parser,
in `xaqu-usb-host`'s `report` module (`src/lib/drivers/usb/usb-host/src/report.rs`),
so this crate does not keep a second one.

| module | what it does | what it does *not* know |
| --- | --- | --- |
| `descriptor` | reads the HID report descriptor: the application collection it declares, the input and output reports, their ids and their sizes, and how many reports an image of a given size takes | the image's width, height and dpi — not in the descriptor, not published |
| `packet` | the framing of a report into a packet: an id byte, a length byte, the payload | what any packet *means* |
| `stream` | the reassembly of an image out of a sequence of packets: which id starts an image, which carries its pieces, which ends it | which ids those are, and how big the image is — both are parameters, not defaults |
| `vendor` | the table of vendor opcodes, **empty**, with the plumbing to fill it | every opcode; a request built from the empty table fails with `Error::UnknownOpcode` |

Deliberate design points:

* The framing sizes (`ID_SIZE`, `LENGTH_SIZE`) are the sizes *observed* on
  the device, and they are values a caller passes in (`packet::Layout`),
  not constants the code assumes, because a firmware revision may frame
  its reports differently.
* The image geometry (`stream::ImageLayout`) defaults to "unknown"
  (zero), never to a made-up resolution. A frame says whether its bytes
  and its geometry agree (`Frame::is_complete`).
* The packet ids that start, carry and end an image are parameters with
  **no** default triple: a default would be a claim about the device that
  nobody has checked.

## 3. The knowledge gap

Everything a fingerprint reader must *ask* the sensor is vendor-specific,
and the vendor protocol is closed. Nothing below is known:

| command | what it is for | opcode |
| --- | --- | --- |
| `FingerPresence` | is a finger on the sensor? | **unknown** |
| `CaptureStart` | start a capture | **unknown** |
| `CaptureStop` | stop a capture | **unknown** |
| `Enroll` | enroll the captured finger | **unknown** |
| `Match` | match the captured finger | **unknown** |
| `StoreTemplate` | store an enrolled template | **unknown** |
| `LoadTemplate` | load an enrolled template | **unknown** |

Unknown as well: the shape of the bytes that go with each opcode, the
framing of a vendor request on the wire, the pixel format of the image
(8-bit greyscale is what the reassembly assumes, and it is an assumption),
and the image's geometry.

This gap is written as code, not as a note. `vendor::VendorTable::empty()`
is the table; `vendor::VendorTable::request()` refuses every command with
`Error::UnknownOpcode`, and `vendor::Builder::build()` refuses to make a
table at all until every command has an opcode. A driver built on this
crate therefore *cannot* pretend to enroll a finger: it is stopped by the
type, at run time, with a named error. That is testable, and the tests do
it.

## 4. How a template store would be keyed, and the plan

A template store is not written yet; this is the shape it would take, and
the decisions it would be built on.

**The key** is the pair *(account, finger slot)*:

* *account*: the system's stable account identifier (the uid the login
  and the session already carry) — not the account's name, which can be
  renamed, and not the hardware id of the finger, which the vendor's
  template format defines and this tree cannot see;
* *finger slot*: the index of the finger within the account (0, 1, 2, …),
  so that several fingers of one account are distinct templates.

**The layout** would be a per-account directory under the system's
writable state, one file per slot:

```
/var/lib/xaqu/fingerprint/<uid>/<slot>.tpl
```

Each file carries a small header of *our* own — a magic, a version, the
slot number, the uid, and the length of what follows — around the
vendor's template bytes, which are opaque to us: we can store them and
hand them back, and we cannot read them, check them or verify them. The
header is what a future version of this tree can migrate on; the bytes
inside are the vendor's.

**At rest**, a template is biometric data and is treated as a secret: the
plan is to encrypt the file with a key derived per account from the
system's own key material (the tree's `xaqu-crypto` area), never to leave
it in clear text, and never to hand a template to anything outside the
account it belongs to. A template store that cannot be read by the next
process on the machine is not a template store.

**The plan**, in order:

1. **Capture the traffic.** On the target laptop, dump the USB traffic
   between the host and the sensor while it is asked, in turn, for
   presence, a capture, an enrollment and a match. This is what turns the
   unknown column of §3 into known values — a capture, not an inference.
2. **Fill the table.** Put the opcodes into `vendor.rs` as a
   `VendorTable` built at run time from the capture, together with the
   wire shape of a request (the one thing this crate deliberately does
   not encode yet).
3. **Confirm the framing.** Use the capture to settle the packet ids, the
   length semantics (`LengthCovers::Payload` versus
   `LengthCovers::WholeReport`), the report sizes and the image geometry,
   and replace the parameters in `stream::StreamLayout` and
   `stream::ImageLayout` with what was observed.
4. **Write the ring-3 driver.** A program under
   `src/user/system/native/drivers/` that claims the USB interface and
   talks to the sensor, per `docs/DRIVER-MODEL.md`: no kernel change, no
   kernel rebuild.
5. **Then, and only then, the store.** The template store and the
   enrolment path come after the opcodes are known, because a store of
   templates the driver cannot produce would be an empty directory with a
   schema.

## 5. What is not implemented, and what is not claimed

* **The vendor protocol.** No opcode, no request encoding, no command
  dispatch. The table is empty and says so in the type.
* **Enrolment, matching, presence.** Not implemented; they are the
  commands of §3.
* **A template store.** Not written; §4 is the plan, not the code.
* **The driver program.** No ring-3 program claims the device yet; this
  crate is the logic such a program would call.
* **The image geometry and the pixel format.** Parameters with documented
  defaults of "unknown"; the reassembly assumes 8-bit greyscale and says
  so.
* **Verification on hardware.** Nothing here has been run against the
  real sensor. The tests use hand-written byte fixtures, which check that
  the bytes are read as this crate says they are read — not that the
  sensor's firmware agrees.
* **Any claim that a finger can be enrolled or matched on Xaqu today.**
  There is none, and there will not be one until step 2 of §4 is done.
