# Instructions Projet Rust - Architecture Hexagonale

## Contexte du Projet
Ce projet est un Workspace Rust structuré selon la Clean Architecture (Hexagonale).
Il est composé de plusieurs crates :
- `libs/ferrisquote-domain` : Le cœur métier (Entities, Ports, Errors). **Aucune dépendance externe lourde** (DB, HTTP).
- `libs/ferrisquote-postgres` : Adaptateur d'infrastructure (Implémentation des Repository via SQLx).
- `libs/ferrisquote-auth` : Logique d'authentification et gestion des identités.
- `apps/ferrisquote-api` : Point d'entrée de l'application (Axum, Handlers, DTOs, Injection de Dépendances).

## Commandes Principales
- Build : `cargo build`
- Test : `cargo test`
- Run API : `cargo run -p ferrisquote-api`
- Lint : `cargo clippy -- -D warnings`
- Format : `cargo fmt`
- Check DB (SQLx) : `cargo sqlx prepare` (si utilisation de query macros)

## Règles d'Architecture

### 1. Séparation des Responsabilités
- **Domain First :** La logique métier réside UNIQUEMENT dans `libs/ferrisquote-domain`. Elle ne doit jamais dépendre de `postgres` ou `axum`.
- **Ports & Adapters :** Le domaine définit des traits (Ports) pour les accès données. `libs/ferrisquote-postgres` implémente ces traits.
- **Dependency Injection :** L'injection se fait manuellement dans le `main.rs` de l'API. Ne pas introduire de framework de DI complexe.

### 2. Gestion des Erreurs
- **Libraries (Domain/Infra) :** Utiliser `thiserror` pour définir des énums d'erreurs typées et précises.
- **Applications (API) :** Utiliser `anyhow` pour la gestion d'erreurs au niveau top-level (main), mais mapper les erreurs de domaine vers des réponses HTTP appropriées dans les handlers.
- **Pas de Panic :** Interdiction d'utiliser `.unwrap()` ou `.expect()` sauf dans les tests ou au démarrage de l'application (configuration).

### 3. Standards de Code (Rust Idiomatic)
- **NewType Pattern :** Utiliser des types forts pour les IDs (ex: `FlowId(Uuid)`) plutôt que des `String` ou `Uuid` nus.
- **Async/Await :** Tout le code I/O est asynchrone (Tokio).
- **SQLx :** Utiliser `sqlx` sans ORM. Préférer les requêtes SQL explicites. Utiliser `PgPool` encapsulé dans des Repositories.
- **Tracing :** Utiliser la crate `tracing` pour les logs (info!, error!, instrument).

### 4. Structure des Handlers (API)
- Les handlers doivent recevoir un `State<AppState>` contenant les services.
- Toujours utiliser des DTOs (`CreateRequest`, `Response`) pour les entrées/sorties API. Ne jamais exposer les entités du domaine directement en JSON.
- Utiliser le trait `Validate` (crate `validator`) pour valider les DTOs en entrée.

## Exemple d'Implémentation Repository (Pattern attendu)
```rust
impl FlowRepository for PostgresFlowRepository {
    fn get_flow(&self, id: FlowId) -> impl Future<Output = Result<Flow, DomainError>> + Send {
        async move {
            // Utilisation de sqlx::query_as! ou query_as
            // Mapping manuel de la structure DB vers l'entité Domain
        }
    }
}
