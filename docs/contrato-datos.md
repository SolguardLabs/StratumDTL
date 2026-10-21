# Contrato de datos JSON

Las estructuras públicas se serializan con `serde`. Los importes y participaciones se transmiten como enteros sin signo; el consumidor debe usar enteros de precisión suficiente y no convertir saldos grandes a `Number` de JavaScript sin validar el rango seguro. `Epoch` y los identificadores mantienen sus tipos de dominio en Rust.

## Pronóstico de liquidez

`LiquidityForecast` contiene `scenario`, `starting_reserve`, `stressed_reserve`, `total_commitments`, `total_available`, `total_shortfall` y cuatro entradas en `tranches`, en el orden público senior, junior, seguro, excedente. Cada `TrancheCoverage` incluye reserva estresada, suelo, disponible, comprometido, excedente, faltante y cobertura en bps. El nombre del escenario procede de la entrada y debe conservarse junto con la política usada.

Ejemplo abreviado de un tramo (los enteros son unidades mínimas del activo):

```json
{
  "tranche": "Junior",
  "stressed_reserve": 600000,
  "policy_floor": 330000,
  "available": 270000,
  "committed": 500000,
  "surplus": 0,
  "shortfall": 230000,
  "coverage_bps": 5400
}
```

La forma exacta de los tipos nuevos se puede comprobar ejecutando `cargo run --quiet --locked --example liquidity_forecast`. `fully_covered()` es un método calculado, no un campo JSON. Las cifras de `total_available` y `total_shortfall` son sumas de tramos después de aplicar sus respectivos suelos; no deben recalcularse restando compromisos del total agregado porque eso permitiría compensaciones entre tramos.

## Evolución de esquema

Mantenga consumidores tolerantes a campos adicionales y estrictos con unidades, nombres de tramo y versión. Guarde la versión de la biblioteca junto a cada captura destinada a auditoría. La serialización es una representación de un estado puntual; la persistencia o rehidratación transaccional queda fuera de este contrato.
