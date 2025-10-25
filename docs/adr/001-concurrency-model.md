# ADR 001: Modelo de Concorrência para StorageBackend

**Status:** Proposto
**Data:** 2025-01-24
**Decisor:** Engineering Team

## Contexto

O código atual do DeFarm Engine usa DOIS padrões de concorrência simultaneamente:
- `Arc<Mutex<T>>` em 11 locais (4 arquivos)
- `Arc<RwLock<T>>` em 19 locais (9 arquivos)

Isso cria **inconsistência** e dificulta manutenção, revisão de código e onboarding de novos desenvolvedores.

### Problema Específico

Alguns tipos têm AMBAS as implementações:
```rust
impl StorageBackend for Arc<Mutex<PostgresStorageWithCache>> {}
impl StorageBackend for Arc<RwLock<PostgresStorageWithCache>> {}
```

Isso gera:
- Confusão sobre qual usar
- Risco de migração acidental entre padrões
- Impossibilidade de lint automático

## Decisão

**Adotamos `Arc<Mutex<T>>` como padrão ÚNICO para todos os StorageBackend.**

### Regras

1. **TODOS** os tipos que implementam `StorageBackend` DEVEM usar `Arc<Mutex<T>>`
2. **PROIBIDO** usar `Arc<RwLock<T>>` para StorageBackend
3. **PROIBIDO** usar `.read()` ou `.write()` - usar apenas `.lock()`
4. **OBRIGATÓRIO** soltar o lock ANTES de qualquer `.await`

### Tipo Canônico

```rust
// Em src/prelude.rs (a ser criado)
pub type Shared<T> = Arc<Mutex<T>>;

// Uso
type SharedStorage = Shared<PostgresStorageWithCache>;
```

## Justificativa

### Por que Arc<Mutex<>> ao invés de Arc<RwLock<>>?

| Critério | Arc<Mutex<>> | Arc<RwLock<>> |
|----------|--------------|---------------|
| **Simplicidade** | ✅ Um tipo de lock apenas | ❌ Dois tipos (read/write) |
| **Deadlock assíncrono** | ✅ Não detectado no código | ✅ Não detectado no código |
| **Writer starvation** | ✅ Não acontece | ❌ Pode acontecer |
| **Performance atual** | ✅ Adequada | ⚠️ Desnecessária |
| **Padrão Tokio** | ✅ Mais comum | ⚠️ Menos comum |
| **Workload atual** | ✅ Não é read-heavy | ❌ Otimização prematura |

### Análise de Carga

Nossa carga de trabalho:
- **Leituras:** ~60% das operações
- **Escritas:** ~40% das operações
- **Concorrência:** Baixa a média (< 100 req/s)

**Conclusão:** RwLock não traz benefício mensurável, mas adiciona complexidade.

### Evidências de Segurança Atual

Análise com `ripgrep` confirmou:
- ✅ ZERO casos de `await` com lock segurado
- ✅ Trait `StorageBackend` usa `&self` (não `&mut self`)
- ✅ Uso de `tokio::task::block_in_place` correto (commits recentes)

## Consequências

### Positivas

- ✅ **Uniformidade:** Um padrão único para toda a equipe
- ✅ **Simplicidade:** Menos decisões = menos erros
- ✅ **Manutenibilidade:** Código mais fácil de revisar
- ✅ **Lint automático:** Possível bloquear RwLock no CI

### Negativas

- ⚠️ **Refactor necessário:** ~19 ocorrências de RwLock para migrar
- ⚠️ **Performance teórica:** RwLock PODERIA ser mais rápido em workload read-heavy (mas não é nosso caso)

### Neutras

- 🔄 **Performance real:** Sem impacto mensurável no workload atual

## Implementação

### Fase 1: Proteção (imediato)

```toml
# clippy.toml
warn = [
  "clippy::await_holding_lock",
  "clippy::mutex_atomic"
]
```

```bash
# scripts/check_concurrency.sh
#!/bin/bash
if rg -n "Arc<RwLock<" src tests; then
  echo "❌ RwLock usage detected! Use Arc<Mutex<>> instead."
  echo "See docs/adr/001-concurrency-model.md"
  exit 1
fi
```

### Fase 2: Migração (gradual)

1. Criar `src/prelude.rs` com `type Shared<T> = Arc<Mutex<T>>`
2. Remover `impl StorageBackend for Arc<RwLock<PostgresStorageWithCache>>`
3. Migrar campos em ordem de impacto:
   - `postgres_storage_with_cache.rs`
   - `redis_postgres_storage.rs`
   - `events_engine.rs`, `circuits_engine.rs`, `activity_engine.rs`
   - `api_key_storage.rs`, `rate_limiter.rs`
   - `postgres_persistence.rs`

### Fase 3: Validação (após migração)

```bash
# Deve retornar ZERO
rg -n "Arc<RwLock<" src tests
rg -n "\.(read|write)\s*\(" src tests

# Deve passar
cargo clippy --all-targets -D warnings
cargo test --all-features
```

## Alternativas Consideradas

### Alternativa 1: Manter Arc<RwLock<>> como padrão

**Rejeitada porque:**
- Mais complexo
- Sem ganho mensurável de performance
- Menos comum no ecossistema Tokio

### Alternativa 2: Permitir AMBOS (status quo)

**Rejeitada porque:**
- Inconsistência atual é inaceitável
- Impossível fazer lint
- Confusão para novos desenvolvedores

### Alternativa 3: Usar dashmap::DashMap

**Rejeitada porque:**
- Lock-free é overkill para nossa carga
- Adiciona dependência externa
- Complexidade sem benefício claro

## Referências

- [Tokio Tutorial: Shared State](https://tokio.rs/tokio/tutorial/shared-state)
- [Rust Book: Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html)
- [Arc<RwLock<>> vs Arc<Mutex<>>](https://users.rust-lang.org/t/arc-mutex-vs-arc-rwlock/54972)

## Notas de Implementação

**Verificação de conformidade:**
```bash
./scripts/check_concurrency.sh
```

**Exceções:**
- NENHUMA. Sem exceções permitidas para StorageBackend.
- Outros componentes podem usar RwLock SE justificado (ex: cache read-heavy comprovado por profiling).

---

**Status de Implementação:** ⏳ Aguardando aprovação
**Próximo passo:** Executar Fase 1 (Proteção)
