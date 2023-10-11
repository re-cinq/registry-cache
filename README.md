### Features

1. Caches only the container image layers, everything else is forwarded to the upstream registry (Authentication, Manifests, Index, Referrers, etc...). This helps also to revalidate with upstream in case a manifest for the same image tag was overwritten (latest tag anyone ?)
Https support
2. Stores the blobs in a temporary file, calculate their digest, to make sure the data is not corrupted and if the data is valid the file is moved (linux atomic operation)
3. Zero copy for both cases:
    - when serving from the cache
    - when serving from upstream
4. Low CPU and memory consumption when blobs are served from the cache (when the content is streamed from upstream, because of point 2. the hash calculation is more CPU intensive)
5. Parallel processing of blob storage
6. Clean shutdown so that in case there are some files still being written the process waits for them to be fully persisted before exiting
7. Support for multiple upstream registries based on the hostname (map the hostname of the cache instance to upstream hostname)
8. Prometheus metric:
    - requests
    - upstream requests
    - cached requests
    - cpu and memory consumption (when running in Linux only - does not work in MacOS because it lacks the /proc/ folder)

### Security:
- The pull-through cache does not implement any authentication for the stored blobs, for everything else it relies on the upstream registry, this means that an attacker can potentially download specific container layer by knowing their digest

### Example config
```YAML
api:
  hostname: "0.0.0.0"
  tls_key: "private key file location"
  tls_cert: "public key file location"

upstreams:
  - host: "192.168.20.123:8080"
    registry: "index.docker.io"
    port: 443
    schema: "https"

storage:
  folder: "/tmp/cache"
```