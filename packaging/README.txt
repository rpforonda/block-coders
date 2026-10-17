BLOCK CODERS - a blocky coding adventure for kids (ages ~8-12)
===============================================================

INSTALL
  1. Unpack this folder anywhere.
  2. Open a terminal in it and run:     ./install.sh
     (To install for every user on the computer:  sudo ./install.sh --system)
  3. Look for "Block Coders" in your app menu.

You can also just run ./block-coders straight from this folder without installing.

REQUIREMENTS
  64-bit Linux with glibc 2.34 or newer (Ubuntu 22.04+, Debian 12+, Fedora 35+,
  Linux Mint 21+, Arch/Manjaro/CachyOS), working graphics (OpenGL), and the
  ALSA sound library (libasound2 / alsa-lib) - almost every desktop has these.

SAVED PROGRESS
  Each player's stars and programs are saved in ~/.local/share/block-coders/
  To move a kid's progress to another computer, copy that folder.

UNINSTALL
  ./uninstall.sh            (add --system if you installed with --system)
  ./uninstall.sh --remove-saves   also deletes all saved progress

PARENT SHORTCUTS
  Ctrl+U on the level screen      unlock every level
  Ctrl+Shift+S inside a level     load a working solution (if you need to help)
