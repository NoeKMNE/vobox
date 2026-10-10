# Vobox

Vobox est une application de bureau distincte d’AgroBox. Elle conserve le moteur
fonctionnel et propose un tableau de bord pour accéder aux fonctionnalités existantes.

## Développement

Prérequis : Node.js/npm. Pour lancer l’application de bureau, installez également Rust.

```sh
npm ci
npm run dev
npx tauri dev
```

## Publication et mises à jour

Les releases Vobox sont publiées dans le dépôt public
[`NoeKMNE/vobox`](https://github.com/NoeKMNE/vobox). Un push sur la branche `publish`
déclenche automatiquement une release : incrément de version, compilation signée pour
Windows/macOS/Linux, génération du manifeste `stable.json`, puis publication des
installateurs et du manifeste dans GitHub Releases. L’application vérifie ce manifeste
au démarrage et télécharge les mises à jour depuis cette release publique.

Configurez les secrets suivants dans **Settings → Secrets and variables → Actions** du
dépôt GitHub :

- `TAURI_SIGNING_PRIVATE_KEY` : contenu de la clé privée créée pour Vobox;
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` : uniquement si la clé privée est régénérée avec
  une phrase secrète (la clé actuelle n’en a pas; laisse ce secret absent).

La clé publique correspondante doit être renseignée dans `plugins.updater.pubkey` de
`src-tauri/tauri.conf.json`. Ne commitez jamais la clé privée dans le dépôt.

Pour une release manuelle, utilisez **Actions → Release → Run workflow** sur la branche
`publish` et saisissez un tag au format `vX.Y.Z`.
