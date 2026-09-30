# P3 — Infrastructure et policy as code

Cette partie décrit le *terrain* (le cluster et le déploiement de l'app) en code,
scanne cette description pour les mauvaises configurations, et impose au cluster des
règles qui rejettent tout manifest non conforme.

Trois briques : IaC (Terraform) · scan IaC (checkov) · policy as code (Kyverno).

## Brique 1 — Infrastructure as Code (Terraform)

`confs/main.tf` décrit, via le provider `kubernetes`, les ressources déployées dans
le cluster k3d local :

- un **namespace `dev`** ;
- un **Deployment `aegis`** durci, qui fait tourner l'image de P1.

Durcissement appliqué au workload (au niveau Kubernetes, complémentaire de l'image) :
- `runAsNonRoot: true` + `runAsUser: 65532` (verrouille le non-root)
- `readOnlyRootFilesystem: true` (aucune écriture sur le disque du conteneur)
- `allowPrivilegeEscalation: false` (pas de gain de privilèges à l'exécution)
- `capabilities: drop ALL` (retrait de toutes les capabilities Linux)
- `resources.limits` + `requests` (plafond CPU/mémoire)
- `liveness` et `readiness` probes sur `/health`

## Brique 2 — Scan de l'IaC (checkov)

Le `main.tf` est scanné par checkov, en local et dans le pipeline CI (job `checkov`
de `.github/workflows/ci.yml`, ciblé sur `p3/confs`, gate bloquant).

Résultat : **25 passed, 0 failed, 2 skipped**. Les deux règles écartées le sont avec
une justification écrite dans le `main.tf` (`#checkov:skip=...`) :
- `CKV_K8S_43` (image par digest) — l'image est importée localement dans k3d ; le pin
  par digest sera appliqué en P4 avec le registry GHCR.
- `CKV_K8S_15` (imagePullPolicy Always) — inadapté à une image locale k3d ; géré en P4.

## Brique 3 — Policy as code (Kyverno)

Kyverno vit dans le cluster et intercepte chaque admission : il **refuse** tout Pod
non conforme, quelle qu'en soit la source. Deux ClusterPolicy en mode `Enforce`
(`confs/policies/`) :

- **`require-non-root.yaml`** — rejette un conteneur sans `runAsNonRoot: true`.
- **`require-resource-limits.yaml`** — rejette un conteneur sans limites CPU/mémoire.

Distinction avec checkov : checkov vérifie *mes* fichiers pendant le développement ;
Kyverno est une loi du cluster qui s'applique à *tout* manifest à l'admission, même
ceux que je n'ai pas écrits.

## Reproduire (sur une machine avec Docker, k3d, kubectl, terraform)

```bash
# 1. cluster local
k3d cluster create aegis

# 2. image de P1 disponible dans le cluster
docker build -f p1/confs/Dockerfile -t aegis:0.1.0 p1
k3d image import aegis:0.1.0 -c aegis

# 3. infra (namespace + deployment durci)
cd p3/confs && terraform init && terraform apply

# 4. policy engine + règles
kubectl create -f https://github.com/kyverno/kyverno/releases/download/v1.13.4/install.yaml
kubectl apply -f policies/
```

## Démonstration (les règles bloquent)

App conforme acceptée :
```bash
kubectl get pods -n dev        # aegis-... Running
```

Pod non conforme (root) rejeté par la règle 1 :
```bash
kubectl run test-root --image=nginx -n dev
# -> denied : require-run-as-non-root
```

Pod conforme au non-root mais sans limites, rejeté par la règle 2 :
```bash
cat <<'EOF' | kubectl apply -f -
apiVersion: v1
kind: Pod
metadata:
  name: test-limits
  namespace: dev
spec:
  containers:
    - name: test
      image: nginx
      securityContext:
        runAsNonRoot: true
EOF
# -> denied : require-resource-limits
```
