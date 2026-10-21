# Operación y validación

## Preparación

Use Rust 1.96 y Node.js 24. El proyecto no requiere claves, RPC ni servicios persistentes. `Cargo.lock` se conserva en control de versiones. Valide cambios de código con `bash scripts/ci.sh`; el script ejecuta formato, build, pruebas, Clippy y las pruebas JavaScript. En Windows, ejecute el script en un entorno Unix con ambas herramientas disponibles o reproduzca los comandos de [README](../README.md) por separado.

## Comandos reproducibles

```bash
cargo fmt --all -- --check
cargo build --all-targets --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
node --test "tests/node/*.test.js"
cargo run --quiet --locked --example liquidity_forecast
```

El último comando emite JSON y no modifica archivos. Para comparar escenarios, cambie únicamente `StressScenario` y la lista de `Commitment` del ejemplo o integre esas estructuras en un cliente propio. Conserve juntos la instantánea inicial, política, escenario y resultado: el faltante aislado carece de contexto económico.

## Guía de observabilidad

En cada operación registre identificador de bóveda, época, tipo de evento, importe bruto/neto y resultado de riesgo. Para liquidaciones conserve el detalle `paid_from`; para ventanas de pérdida, el recibo de asignación y el evento de publicación. Confronte periódicamente `total_reserve`, suministros de participaciones y entradas del diario mediante `InvariantSuite`, `AuditEngine` y `ReconciliationEngine`. Una discrepancia exige detener la integración externa y preservar la secuencia original; no se corrige reescribiendo instantáneas.

## Publicación de versión

La rama `main` contiene la historia de integración. `production` y el tag anotado `v1.0.0` deben apuntar exactamente al commit publicado. El workflow `release-integrity` comprueba esa identidad y la existencia de la documentación crítica. Una release identifica el código verificado; por sí misma no despliega servicios ni cambia fondos.
