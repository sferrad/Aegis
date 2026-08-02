# P2 — Pipeline CI sécurisé

Pipeline GitHub Actions déclenché à chaque `push` et `pull_request`, qui refuse de
livrer l'application si une vérification de sécurité échoue.

Le workflow est défini dans `.github/workflows/ci.yml` (à la racine du dépôt, seul
emplacement lu par GitHub Actions).

## Les cinq étapes (jobs)

Chaque job tourne en parallèle sur une machine Ubuntu neuve, produit un rapport
conservé comme artifact, et fait échouer le build si une trouvaille dépasse le seuil.

| Job          | Rôle                        | Outil    | Seuil de blocage        |
|--------------|-----------------------------|----------|-------------------------|
| `hadolint`   | Lint du Dockerfile          | hadolint | toute erreur            |
| `gitleaks`   | Scan de secrets (repo + historique) | gitleaks | tout secret détecté |
| `semgrep`    | SAST (analyse du code source) | semgrep | toute trouvaille        |
| `sca`        | Dépendances vulnérables     | trivy fs | HIGH, CRITICAL          |
| `image-scan` | Vulnérabilités de l'image   | trivy image | HIGH, CRITICAL       |

Distinction importante : `hadolint` juge la **qualité d'écriture** du Dockerfile,
tandis que `image-scan` inspecte le **contenu réel** de l'image construite (paquets
système et leurs CVE). `sca` regarde les dépendances applicatives (`Cargo.lock`),
`semgrep` regarde le code écrit par le développeur. Les périmètres sont complémentaires.

## Choix de conception

- **SAST ciblé sur `p1/app/src`** : le SAST analyse le code de l'application, pas la
  plomberie du dépôt (workflows, Dockerfile), déjà couverte par hadolint et P3.
- **Scan d'image via la CLI Trivy** (et non l'action) : évite le cache de base CVE de
  l'action, qui rendait le résultat non reproductible. Le scan CI utilise donc la même
  base à jour que le scan local. Le job génère d'abord le rapport, puis applique le gate.
- **Seuils HIGH/CRITICAL** pour les scans de vulnérabilités : on bloque sur le grave,
  on ignore le bruit LOW/MEDIUM.
- **Zéro dépendance applicative** (choix de P1) : la surface SCA est nulle par conception.

## Rapports (artifacts)

Chaque exécution attache ses rapports téléchargeables : `semgrep-report`,
`sca-report`, `image-scan-report`, et le résumé gitleaks (`gitleaks-results.sarif`).

## Blocage de merge

Une *branch protection* (ruleset GitHub) protège `main` :
- Require a pull request before merging
- Require status checks to pass : les 5 jobs doivent être verts

Conséquence : une PR dont un scanner rougit a son bouton *Merge* désactivé.

## Démonstration (passage / échec)

Run qui passe : tout push propre sur `main` → les 5 jobs verts.

Run qui échoue volontairement (secret détecté) :

```bash
git checkout -b test-secret-leak
printf 'github_pat = "ghp_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"\n' > p2/leaked_secret.txt
git add p2/leaked_secret.txt
git commit -m "test: secret detectable"
git push -u origin test-secret-leak
```

→ le job `gitleaks` passe en échec et bloque le build.

Nettoyage après la démo :

```bash
git checkout main
git branch -D test-secret-leak
git push origin --delete test-secret-leak
```

> La valeur `ghp_...` ci-dessus est factice : bon format pour déclencher la règle,
> aucun accès réel. Un faux secret ne doit jamais rester sur `main`.

## Reproduire localement

Les scans d'image et de dépendances peuvent être rejoués sur un poste avec Docker :

```bash
# depuis p1/ : build de l'image
docker build -f confs/Dockerfile -t aegis:ci .

# scan d'image (mêmes conditions que le CI)
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
  aquasec/trivy image --severity HIGH,CRITICAL --scanners vuln aegis:ci

# scan des dépendances
docker run --rm -v "$PWD:/src" \
  aquasec/trivy fs --severity HIGH,CRITICAL /src/app
```