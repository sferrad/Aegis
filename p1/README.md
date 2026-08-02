# P1 — Application et image durcie

Service HTTP minimal écrit en Rust (zéro dépendance externe) et son image Docker durcie.
L'application est volontairement triviale : ce qui est évalué, c'est la chaîne de sécurité
construite autour, pas ses fonctionnalités.

## Routes

- `GET /` → petit payload JSON incluant un champ `version`
- `GET /health` → liveness check (utilisé par le HEALTHCHECK)

## Durcissement appliqué

- Build multi-stage : compilation dans un stage « builder », image finale repartant d'une base vide
- Image finale distroless (`gcr.io/distroless/cc-debian12`), ~9 MB, sans shell ni gestionnaire de paquets
- Exécution en non-root (uid numérique `65532`)
- Bases pinnées par digest (`@sha256:...`) — aucun tag flottant, build reproductible
- Aucun secret, token ou credential dans les couches de l'image
- HEALTHCHECK intégré : le binaire se sonde lui-même via un sous-mode (`/aegis healthcheck`),
  car l'image distroless ne contient ni `curl` ni shell
- Un seul port exposé (`8080`)
- Passe le linter hadolint sans erreur

## Arborescence

```
p1/
├── app/                 # code source Rust
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src/main.rs
├── confs/
│   └── Dockerfile       # build multi-stage durci
├── Makefile
└── README.md
```

## Reproduire (sur une machine propre)

Prérequis : Docker.

```bash
make build      # construit l'image durcie
make run        # lance le conteneur, expose le port 8080
make clean      # supprime l'image
```

## Vérifier

Avec le conteneur lancé (`make run`), dans un autre terminal :

```bash
curl localhost:8080          # -> {"service": "aegis", "version": "0.1.0"}
curl localhost:8080/health   # -> {"status": "healthy"}
```

Vérifier le healthcheck intégré :

```bash
docker run --rm -d --name aegis-test -p 8080:8080 aegis:0.1.0
sleep 10
docker ps                    # STATUS doit afficher (healthy)
docker stop aegis-test
```

Vérifier le lint du Dockerfile :

```bash
docker run --rm -i hadolint/hadolint hadolint - < confs/Dockerfile
echo $?                      # doit afficher 0 (aucun finding)
```