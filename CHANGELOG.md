# Journal des modifications

## 0.3.0 — 16 novembre 2026

### Ajouts
- Mises à jour automatiques : SXM Boot vérifie au démarrage (puis toutes les 6 heures) si une nouvelle version est publiée.
- Petite fenêtre de mise à jour, extensible pour voir la liste des nouveautés, corrections et améliorations de sécurité.
- Installation en un clic : Windows s'installe et redémarre seul, Fedora installe le paquet via dnf (mot de passe demandé), macOS ouvre le .dmg.
- Bouton « Rechercher des mises à jour » dans la page À propos.

### Sécurité
- Les mises à jour ne sont acceptées que depuis le dépôt officiel GitHub, et chaque installateur est vérifié avec sa somme SHA256 publiée par GitHub.

### Corrections
- Le numéro de version affiché vient maintenant directement de l'application.

## 0.2.0 — 15 novembre 2026

### Ajouts
- Ajout de CachyOS Desktop 260809 au catalogue, via le CDN officiel.
- Ajout de la somme SHA256 publiée dans la documentation officielle CachyOS.

### Sécurité et fiabilité
- Les opérations de gestion des ISO vérifient que le point de montage est bien celui d'une partition Ventoy détectée sur une clé USB.
- Validation stricte des noms de fichiers ISO/IMG et des domaines HTTPS autorisés pour les téléchargements du catalogue.
- Suppression des commandes shell interpolant les chemins de périphériques pour le démontage Linux et le lancement de Ventoy.
- Une vérification SHA256 demandée ne peut plus échouer silencieusement ; si aucune somme officielle n’est configurée (GParted Live), l’application demande explicitement de désactiver l’option pour poursuivre sans contrôle.
- Suppression des fichiers temporaires en cas d'échec ou d'annulation, contrôle de la taille téléchargée et refus de remplacer silencieusement une ISO déjà présente.
- Activation d'une politique CSP locale pour l'interface Tauri.

### Corrections avant publication
- Linux : la préparation d'une clé vierge refait le changement de dossier dans la commande lancée par `pkexec` (qui revient toujours dans le dossier de root), sans insérer de texte dans un shell.
- GParted Live (sans somme SHA256 officielle) n'empêche plus la création : l'ISO est ajoutée avec la mention « non vérifiée » dans le journal, les autres restent vérifiées.
- CSP : autorisation du canal IPC de Tauri (`ipc:`), pour éviter le mode de secours plus lent.
- Retrait d'un miroir inutilisé de la liste des domaines autorisés.
- CachyOS placé en tête du catalogue (n°1 sur DistroWatch).
- Traductions anglaises ajoutées pour les nouveaux messages.

### Validation
- Syntaxe JavaScript vérifiée avec Node.js.
- Compilation Rust vérifiée (Linux) ; Windows et macOS compilés par GitHub Actions.
