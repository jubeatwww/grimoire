# Deploying Grimoire

This directory is what a cluster needs to run Grimoire, and nothing from the rest of the repo:
`chart/` (a Helm chart) and `app.yaml`, which describes it to
[cairn](https://github.com/jubeatwww/cairn) as a plugin.

```
                  <hostname> (LAN only)
                          │
                 Traefik Ingress (TLS)
                ┌─────────┴─────────┐
              /api                  /
                │                   │
        grimoire-server         grimoire-web
        (Rust, port 3000)       (nginx + Vite build)
                │
        ┌───────┴────────────┐
        │                    │
   postgres-rw         NFS <nas.server>:<libraryPath>
   (same namespace)    at /mnt/games, asset cache included
```

## Install as a cairn plugin

From the site directory, clone only this directory (the source stays on GitHub):

```sh
git clone --filter=blob:none --sparse git@github.com:jubeatwww/grimoire.git plugins/grimoire
git -C plugins/grimoire sparse-checkout set deploy
```

Add the entry to `site.yaml`, then fill the secret and apply:

```yaml
apps:
  grimoire:
    app: grimoire/deploy
    hostname: grimoire.example.com    # its DNS record points at the node's LAN IP
    libraryPath: /volume1/library     # NFS export on nas.server with the games
```

```sh
scripts/secret.py fill grimoire       # ghcr.token: a classic PAT with read:packages
helmfile apply -l app=grimoire        # Postgres, the app, its nightly backup
```

`git -C plugins/grimoire pull` brings changes to the chart; `helmfile apply -l app=grimoire` again
applies them.

What the chart reads besides its own `values.yaml`:

| Value | |
|---|---|
| `hostname` | the Ingress host, LAN only |
| `libraryPath` | the NFS export with the games, mounted read-write at `/mnt/games` |
| `nas.server` | the NFS server |
| `network.lanCidr` | who gets past the `lan-only` middleware |
| `ghcr.token` | pulls the private images |
| Secret `postgres-app` | `uri` for the database, made by the Postgres cluster in the same namespace |

The server creates its tables in the schema `grimoire` on its first start. The database is
dumped every night with cairn's backup; covers and uploaded images are on the NAS under
`<libraryPath>/grimoire-assets`.

## Build and push the images

From the repo root. The node is linux/amd64, so a build on Apple Silicon needs `--platform`:

```sh
docker buildx build --platform linux/amd64 -f Dockerfile.server -t ghcr.io/jubeatwww/grimoire-server:latest --push .
docker buildx build --platform linux/amd64 -f Dockerfile.web -t ghcr.io/jubeatwww/grimoire-web:latest --push .
```

Both use the moving tag `latest`, pulled again on each rollout:

```sh
kubectl -n grimoire rollout restart deploy/grimoire-server deploy/grimoire-web
```
