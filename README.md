# Smart Archive

Un outil de gestion d'archives avec vérification d'intégrité par checksums MD5.

## Installation

```bash
cargo build --release
```

L'exécutable se trouve alors dans `target/release/smart-archive`.

## Utilisation

```bash
smart-archive --exclude $EXCLUDE --quiet $COMMAND
```

### Options globales

- `-q, --quiet` : Désactive la sortie console (les logs sont toujours écrits dans le fichier `.archivage-historique.log`)
- `-e, --exclude $EXCLUDE` : Liste de chemins à exclure (expressions glob UNIX séparées par des virgules)
- `-h, --help` : Affiche l'aide

### Commandes

#### prepare

Prépare un dossier en générant le fichier `.checksums`.

```bash
smart-archive prepare $CHEMIN
```

Exemple :
```bash
smart-archive -e '**/target,**/.git' prepare .
```

Les checksums sont calculées pour chaque fichier de chaque sous-dossier du chemin passé en argument.

#### verify

Vérifie l'intégrité d'un dossier en comparant les fichiers avec les checksums stockés.

```bash
smart-archive verify $CHEMIN
smart-archive verify --interactive $CHEMIN  # Mode interactif pour gérer les différences
```

#### copy

Copie un dossier en vérifiant les checksums. Le dossier source doit déjà contenir un fichier `.checksums` valide.

```bash
smart-archive copy $SOURCE $DESTINATION
```

Exemple :
```bash
smart-archive copy /source /destination
```

Si le dossier de destination existe et n'est pas vide, le dossier source est copié à l'intérieur avec son nom.

## Fonctionnement

1. **Préparation** : La commande `prepare` parcourt tous les fichiers du dossier (en excluant les fichiers cachés et ceux correspondant aux patterns d'exclusion) et calcule leur checksum MD5, stockant le résultat dans `.checksums`.

2. **Vérification** : La commande `verify` compare les checksums actuels avec ceux attendus. En mode interactif (`-i`), vous pouvez ajouter/supprimer/mettre à jour les entrées.

3. **Copie** : La commande `copy` vérifie l'intégrité du source, copie les fichiers et vérifie à nouveau les checksums après copie.

## Fichier `.checksums`

Format : `{chemin}\t{md5_hash}`

Le fichier est trié alphabétiquement par chemin et placé à la racine du dossier archivé.

## Améliorations

Avant rayon:
```
time ./smart-archive verify /data/alexandru
2026-06-02T15:15:28.861027744+02:00 INFO smart_archive - Vérification terminée, tout est conforme.
./smart-archive verify /data/alexandru  2,95s user 0,19s system 99% cpu 3,139 total
```

Après rayon, la même commande:
```
time ./smart-archive verify /data/alexandru
2026-06-02T15:17:32.295478078+02:00 INFO smart_archive - Vérification terminée, tout est conforme.
./smart-archive verify /data/alexandru  4,37s user 0,25s system 1764% cpu 0,262 total
```

L'amélioration est d'un facteur presque 12 !