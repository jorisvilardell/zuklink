# Instructions Projet Rust - ZukLink (Distributed Streaming)

## Contexte du Projet
Ce projet est une plateforme de streaming distribuée Cloud-Native (Rust Workspace).
L'architecture est de type **"Flat Storage / Smart Receiver"** (Stockage S3 à plat + Coordination P2P).

Il est composé de plusieurs crates :
- `libs/zuklink-yellowpage` : **Cœur de la coordination**. Bibliothèque de Gossip Protocol (UDP/TCP) pour la découverte des membres et le Heartbeat.
- `apps/zuk-bolt` (Sender) : Service d'ingestion **Stateless**. Écrit les données sur S3 (UUID). Ne connaît pas les receivers.
- `apps/zuk-sink` (Receiver) : Service de traitement **Stateful**. Utilise `yellowpage` pour le Consistent Hashing et polle S3.

## Commandes Principales
- Build : `cargo build`
- Test (Unit + Libs) : `cargo test`
- Run Sender : `cargo run -p zuk-bolt`
- Run Receiver : `cargo run -p zuk-sink`
- Infra (Local) : `docker compose up -d`
- Lint : `cargo clippy -- -D warnings`
- Format : `cargo fmt`

## Règles d'Architecture

### 1. Philosophie Distribuée
- **Shared Nothing :** Les services ne partagent AUCUNE base de données (type Redis/SQL). Le seul état partagé est le bucket S3 (Données) et la vue réseau (Gossip).
- **Flat Storage :** S3 est la source de vérité. Le Sender écrit des fichiers avec des noms aléatoires (UUID). Pas de structure de dossiers complexe imposée par le Sender.
- **Smart Receiver :** Toute l'intelligence de répartition de charge réside dans le Receiver (Client-Side Sharding).

### 2. Séparation des Responsabilités
- **Libs vs Apps :** `zuklink-yellowpage` ne doit jamais dépendre de la logique métier de `bolt` ou `sink`. Elle fournit juste une `View` du cluster.
- **Async First :** Tout est asynchrone (Tokio). Les appels S3 et Réseau ne doivent jamais bloquer le thread principal.

### 3. Gestion des Erreurs & Concurrence
- **Erreurs :**
    - `libs/*` : Utiliser `thiserror` pour des erreurs typées (ex: `GossipError`, `DiscoveryError`).
    - `apps/*` : Utiliser `anyhow` pour le top-level.
- **Verrous :** Utiliser STRICTEMENT `tokio::sync::RwLock` (pas `std::sync`) pour les états partagés comme la `ClusterView` dans la Yellowpage.
- **Panic :** Interdit en production. Gérer les timeouts S3 et réseau proprement.

### 4. Standards de Code (Rust Idiomatic)
- **Tracing :** Utiliser la crate `tracing` avec des spans pour suivre le cheminement d'un paquet ou d'un événement Gossip.
- **Configuration :** Utiliser des variables d'environnement (12-Factor App) pour la config (S3 Endpoint, Ports).
- **Types Forts :** Ne pas passer des `String` pour des adresses. Utiliser `SocketAddr` ou des types dédiés `NodeId`.

## Exemple de Logique Sharding (Pattern attendu dans Receiver)

Le Receiver doit filtrer les fichiers S3 sans coordination centrale :

```rust
/// Détermine si ce Receiver doit traiter le fichier donné
/// Basé sur le Consistent Hashing (Rendezvous Hashing ou Modulo simple)
fn should_process_file(filename: &str, my_node_index: usize, cluster_size: usize) -> bool {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    if cluster_size == 0 { return false; }

    let mut hasher = DefaultHasher::new();
    filename.hash(&mut hasher);
    let hash = hasher.finish();

    // Répartition déterministe
    (hash as usize % cluster_size) == my_node_index
}
