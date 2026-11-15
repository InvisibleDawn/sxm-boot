// SXM Boot — © 2026 Laurent Honoré — GPL-3.0-or-later
// Catalogue : systèmes avec lien de téléchargement direct vérifié.
// "sums" = fichier de sommes SHA256 publié à côté de l'ISO (vérification automatique).
window.CATALOG = [
  { id: 'debian', logo: 'img/debian.svg', color: '#C7084F', name: 'Debian',
    desc: 'Debian Stable en Net Install : installation via Internet.',
    desc_en: 'Debian Stable Net Install: installs over the Internet.', version: 'Stable 13.7 · Netinst', size: 0.8e9,
    url: 'https://debian.grena.ge/debian-cd/13.7.0/amd64/iso-cd/debian-13.7.0-amd64-netinst.iso',
    sums: 'https://debian.grena.ge/debian-cd/13.7.0/amd64/iso-cd/SHA256SUMS' },
  { id: 'fedora-mate', logo: 'img/fedora.svg', color: '#3C6EB4', name: 'Fedora MATE',
    desc: 'Fedora avec le bureau MATE : classique, léger et stable.',
    desc_en: 'Fedora with the MATE desktop: classic, light and stable.', version: 'Fedora 45', size: 3.1e9,
    url: 'https://download.fedoraproject.org/pub/fedora/linux/releases/45/Spins/x86_64/iso/Fedora-MATE_Compiz-Live-45-1.4.x86_64.iso',
    sums: 'https://dl.fedoraproject.org/pub/fedora/linux/releases/45/Spins/x86_64/iso/Fedora-Spins-45-1.4-x86_64-CHECKSUM' },
  { id: 'ubuntu-mate', logo: 'img/ubuntumate.svg', color: '#7AA83F', name: 'Ubuntu MATE',
    desc: 'Ubuntu avec l\'environnement MATE : léger, complet et classique.',
    desc_en: 'Ubuntu with the MATE desktop: light, complete and classic.', version: 'Stable 26.10', size: 4.2e9,
    url: 'https://cdimage.ubuntu.com/ubuntu-mate/releases/26.10/release/ubuntu-mate-26.10-desktop-amd64.iso',
    sums: 'https://cdimage.ubuntu.com/ubuntu-mate/releases/26.10/release/SHA256SUMS' },
  { id: 'mint', logo: 'img/linuxmint.svg', color: '#69A84F', name: 'Linux Mint',
    desc: 'Simple, élégante et très facile pour débuter, avec le bureau Cinnamon.',
    desc_en: 'Simple, elegant and beginner-friendly, with the Cinnamon desktop.', version: 'Stable 22.3', size: 2.9e9,
    url: 'https://ftp.nl.freebsd.org/os/Linux/distr/linuxmint/iso/stable/22.3/linuxmint-22.3-cinnamon-64bit.iso',
    sums: 'https://ftp.nl.freebsd.org/os/Linux/distr/linuxmint/iso/stable/22.3/sha256sum.txt' },
  { id: 'gparted', logo: 'img/gparted.svg', color: '#5B6470', name: 'GParted Live',
    desc: 'Outil de gestion et de réparation des partitions de disque.',
    desc_en: 'Tool to manage and repair disk partitions.', version: 'Stable 1.8', size: 0.64e9,
    url: 'https://downloads.sourceforge.net/project/gparted/gparted-live-stable/1.8.0-2/gparted-live-1.8.0-2-amd64.iso' }
];
