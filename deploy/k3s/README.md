# Grimoire on k3s

Topology:

```
                 grimoire.shar0.dev
                          │
                 Traefik Ingress (TLS)
                ┌─────────┴─────────┐
              /api/*               /*
                │                   │
        grimoire-server         grimoire-web
        (Rust, port 3000)       (nginx + Vite build)
                │
        ┌───────┴────────┐
        │                │
   pg-main-rw      grimoire-nas PVC
   (database ns,    (NFS 192.168.1.193:/volume1/games)
    CNPG cluster)
```

## 1. Build & push images

Both images get tagged `latest` for simplicity — swap for a real version
once you start caring about rollbacks.

```bash
# from repo root
GHCR=ghcr.io/<your-github-user>

docker build -t $GHCR/grimoire-server:latest -f Dockerfile.server .
docker build -t $GHCR/grimoire-web:latest    -f Dockerfile.web .

docker push $GHCR/grimoire-server:latest
docker push $GHCR/grimoire-web:latest
```

Then replace `REPLACE_ME` in `20-server.yaml` / `30-web.yaml` with your
GHCR user (or `sed -i "s|REPLACE_ME|$GH_USER|g" 20-server.yaml 30-web.yaml`).

## 2. One-time Postgres prep

The deployment shares `pg-main-app`'s credentials with the rest of the
cluster but stays isolated via the `grimoire` schema. Two things have to
exist on `pg-main` before the server starts:

```bash
# uuid-ossp extension (server migration needs it — needs superuser once)
kubectl exec -it -n database pg-main-1 -- \
  psql -d app -c 'CREATE EXTENSION IF NOT EXISTS "uuid-ossp";'

# `app` user needs CREATE on the database so migrations can create the
# `grimoire` schema. If the role already has CREATE (CNPG default does),
# skip this.
kubectl exec -it -n database pg-main-1 -- \
  psql -d app -c 'GRANT CREATE ON DATABASE app TO app;'
```

## 3. Secrets

### 3a. DB password

The server pod reads `grimoire-db` / key `password`. Cheapest path is to
mirror `pg-main-app`'s password into the grimoire namespace:

```bash
kubectl create namespace grimoire 2>/dev/null

PG_PASSWORD=$(kubectl get secret pg-main-app -n database \
  -o jsonpath='{.data.password}' | base64 -d)

kubectl create secret generic grimoire-db -n grimoire \
  --from-literal=password="$PG_PASSWORD"
```

### 3b. GHCR pull secret

The deployments reference `ghcr-pull`. The convenient path is to copy
the one already in `aetherium`:

```bash
kubectl get secret ghcr-pull -n aetherium -o yaml \
  | grep -v '^\s*\(namespace\|uid\|resourceVersion\|creationTimestamp\|ownerReferences\):' \
  | kubectl apply -n grimoire -f -
```

Or build from scratch (GH PAT with `read:packages`):

```bash
kubectl create secret docker-registry ghcr-pull \
  -n grimoire \
  --docker-server=ghcr.io \
  --docker-username=<your-github-user> \
  --docker-password=<gh-pat> \
  --docker-email=<your-email>
```

## 4. Apply

```bash
kubectl apply -f deploy/k3s/00-namespace.yaml
kubectl apply -f deploy/k3s/10-nas-pv-pvc.yaml
kubectl apply -f deploy/k3s/20-server.yaml
kubectl apply -f deploy/k3s/30-web.yaml
kubectl apply -f deploy/k3s/40-ingress.yaml
```

Watch it come up:

```bash
kubectl get pods -n grimoire -w
kubectl logs -n grimoire deploy/grimoire-server -f
```

First time, the server will hang briefly on `failed to run database
migrations` if Postgres hasn't bound yet — give it ~20s.

## 5. Verify

```bash
# DNS first — your router should already point grimoire.shar0.dev → node IP
curl -I https://grimoire.shar0.dev

# Health round-trip
curl https://grimoire.shar0.dev/api/library/ | head -c 200
```

Then open <https://grimoire.shar0.dev> in a browser and run a Scan.

## Rolling out updates

```bash
docker build -t $GHCR/grimoire-server:latest -f Dockerfile.server .
docker push $GHCR/grimoire-server:latest
kubectl rollout restart deployment/grimoire-server -n grimoire
```

(`imagePullPolicy: Always` + `restart` forces a fresh pull. Switch to
proper version tags when you want hash-pinned rollbacks.)

## Cleanup

```bash
kubectl delete -f deploy/k3s/ --ignore-not-found
# PV is Retain — the actual NFS data stays. Delete the PV manually if
# you really want it gone:
kubectl delete pv grimoire-nas
```
