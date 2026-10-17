# Share with a nearby device

Open your notes on a tablet or phone on the **same network** as your computer — handy for writing on
a touch screen, or for taking a photo straight into a note.

**Your notes stay on the computer.** The other device reads and writes them over your own network,
through the computer, and nothing is sent anywhere else. This is not the phone app: it is the
computer's formicaria, opened in the other device's browser.

## Turning it on

**Settings → Connections → Share with another device**, then **On**. It is **off by default**, and a
change takes effect the next time formicaria starts. Once on, the line beside the switch says whether
other devices can actually reach this computer — a Wi-Fi network that keeps devices apart, or a
firewall, is the usual reason they cannot.

## Choosing what it may see

Under **Which vaults**, tick the notebooks a device may use. A paired device is let into those and
**no others**: it cannot see, search or open anything in the rest. Choose at least one before pairing.

## Pairing a device

1. **Trust this computer first.** The connection is encrypted, so the other device has to accept this
   computer's certificate once. Copy the file whose path is shown to the device, open it, and allow it
   (on an iPad, also **Settings → General → About → Certificate Trust Settings**). **Before you allow
   it, check the device shows exactly the fingerprint printed under the path.** If it shows anything
   else, stop — you are not talking to this computer. Encryption is also what lets the device use its
   microphone.
2. On the other device, open the address shown (it starts with `https://`).
3. Back on the computer, press **Start a pairing code**, and type the code on the other device. A code
   works **once**, and only for the next few minutes.

## If a device is lost

**Disconnect all devices** signs every paired device out at once. Each will need a new code to connect
again.
