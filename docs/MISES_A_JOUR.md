# Mises à jour automatiques Vobox

Vobox vérifie `updates/stable.json` au démarrage et depuis Paramètres. Si une version
plus récente est disponible, l’application affiche les notes de version et propose son
installation. Les téléchargements passent par GitHub Releases et sont vérifiés avec la
clé publique Vobox configurée dans `src-tauri/tauri.conf.json`.

## Publication continue

Le workflow **Auto Release** s’exécute après un push sur `publish` (sauf commits de
publication eux-mêmes). Il incrémente la version patch, crée un tag, compile et signe
Vobox pour Windows/macOS/Linux, puis publie les installateurs et `stable.json` dans une
GitHub Release du dépôt `NoeKMNE/vobox`. Le commit du manifeste sur `publish` porte le
marqueur `[skip release]` pour éviter une boucle.

Pour lancer une publication :

```sh
git push origin publish
```

Une release manuelle est aussi disponible depuis **Actions → Release → Run workflow**
sur la branche `publish`, avec un tag au format `vX.Y.Z`.

## Configuration initiale

La clé de signature est dédiée à Vobox et ne doit jamais être partagée avec un autre
produit.
La clé privée locale se trouve dans `%USERPROFILE%\.tauri\vobox.key`; ne la commitez
jamais.

Dans **Settings → Secrets and variables → Actions** du dépôt public `NoeKMNE/vobox`,
configurez :

- `TAURI_SIGNING_PRIVATE_KEY` : le contenu du fichier `vobox.key`;
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` : seulement si la clé est régénérée avec une
  phrase secrète. Pour la clé actuelle sans mot de passe, laisse ce secret absent.

La clé publique associée est déjà renseignée dans `plugins.updater.pubkey`. Ne remplacez
pas une clé sans mettre à jour simultanément le secret GitHub et la clé publique; sinon,
les installations existantes ne pourront plus vérifier les nouvelles versions.

## Dépannage

- Une version n’est proposée que si le manifeste publié contient une version supérieure
  à celle de l’application installée.
- Si la vérification de signature échoue, vérifiez que les secrets CI et la clé publique
  appartiennent bien à la même paire de clés Vobox.
- Si la mise à jour n’est pas trouvée, vérifiez que `stable.json` est joint à la dernière
  GitHub Release publiée de `NoeKMNE/vobox`.
