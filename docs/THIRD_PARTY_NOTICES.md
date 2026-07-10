# Third-party notices

Touch Manager includes the following separately maintained open-source component in its
macOS release bundle.

## dfu-util 0.11

`dfu-util` is a host-side implementation of USB Device Firmware Upgrade protocols. Touch
Manager invokes it as a separate executable for firmware transfers.

- Project: <https://dfu-util.sourceforge.net/>
- Version: 0.11
- Source archive SHA-256: `b4b53ba21a82ef7e3d4c47df2952adf5fa494f499b6b0b57c58c5d04ae8ff19e`
- License: GNU General Public License, version 2 or later
- Copyright 2005–2009 Weston Schmidt, Harald Welte and OpenMoko Inc.
- Copyright 2010–2021 Tormod Volden and Stefan Schmidt

The complete license text is bundled at `licenses/dfu-util-GPL-2.0.txt`. The exact
corresponding `dfu-util-0.11.tar.gz` source archive is attached to every Touch Manager
macOS release that includes this executable.

## libusb 1.0.30

The bundled `dfu-util` executable statically incorporates `libusb` for USB access.

- Project: <https://libusb.info/>
- Version: 1.0.30
- Source archive SHA-256: `fea36f34f9156400209595e300840767ab1a385ede1dc7ee893015aea9c6dbaf`
- License: GNU Lesser General Public License, version 2.1 or later

The complete license text is bundled at `licenses/libusb-LGPL-2.1.txt`. The exact
corresponding `libusb-1.0.30.tar.bz2` source archive is attached to every Touch Manager
macOS release that includes this library.

Touch Manager itself remains licensed under the MIT License. Third-party components remain
subject to their respective licenses.
