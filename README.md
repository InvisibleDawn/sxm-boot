# SXM Boot

Créateur de clé USB multiboot pour Fedora, avec une interface claire en 3 étapes.
Le moteur de démarrage est Ventoy, que l'appli télécharge elle-même au premier lancement.

## Ce que fait l'appli

1. **Clé USB** : détecte uniquement les clés USB (jamais les disques internes). Une clé vierge est préparée avec Ventoy après confirmation ; le mot de passe administrateur est demandé par polkit.
2. **Systèmes & outils** : galerie par catégorie. Pour chaque système, tu choisis le fichier .iso sur ton PC ou tu colles un lien direct (avec la somme SHA256 si tu l'as). Tu vois aussi les ISO déjà présentes sur la clé et tu peux les supprimer.
3. **Création** : copie ou téléchargement direct sur la clé avec progression, vérification SHA256, annulation et éjection.

## Compiler sur Fedora

```bash
# Dépendances
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel gcc rpm-build
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install tauri-cli --version "^2"

# Lancer en mode développement
cd sxm-boot/src-tauri
cargo tauri dev

# Créer le paquet .rpm
cargo tauri build
sudo dnf install ./target/release/bundle/rpm/SXM\ Boot-0.1.0-1.x86_64.rpm
```

## Version macOS

Le fichier `.dmg` est compilé automatiquement par GitHub Actions (onglet **Actions** du dépôt → dernière exécution → artefact **SXM-Boot-macOS**).

- Sur Mac, SXM Boot détecte les clés, ajoute, télécharge et retire des ISO.
- La **préparation d'une clé vierge (Ventoy) se fait une seule fois sous Linux** : Ventoy n'existe pas pour macOS.
- L'appli n'est pas signée par Apple : au premier lancement, fais clic droit → **Ouvrir**, ou lance `xattr -cr "/Applications/SXM Boot.app"`.

## Personnaliser le catalogue

Modifie `ui/catalog.js`. Pour un téléchargement direct en un clic, ajoute `url` (et si possible `sha256`) à une entrée.

## Notes

- Secure Boot doit être désactivé (ou Ventoy enregistré via MOK).
- Ventoy est stocké dans `~/.local/share/sxm-boot/ventoy/`. Une installation dans `/opt/ventoy/` est aussi reconnue.

## Licence

© 2026 Laurent Honoré. Distribué sous licence GNU GPL v3 ou ultérieure (voir `LICENSE`).
