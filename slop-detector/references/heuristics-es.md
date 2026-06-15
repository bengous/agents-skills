# Heurísticas de escritura IA — Español

Catálogo de tics de lenguaje de la IA en español. Cargar este archivo cuando el
texto auditado esté en español (o ejecutar `score.py --lang es`). Las
heurísticas inglesas de `heuristics.md` no aplican tal cual: cambian los
marcadores, y los anglicismos/calcos se vuelven una señal relevante.

## Principio de gradación (recordatorio)

Igual que en inglés: un tic aislado no es prueba. Las **señales DURAS** (fuga de
asistente, artefactos de modelo, calcos muy específicos) cuentan con una sola
ocurrencia; las **señales BLANDAS** (anglicismos, conectores, intensificadores,
rayas) solo cuentan en racimo — repetición o co-ocurrencia con una señal dura o
un pivote. Ver `references/strictness-modes.md`.

## Categorías

- **assistant-leakage** (DURA, sev 10) — « como modelo de lenguaje », « como asistente de IA », divulgación de identidad o de fecha de corte.
- **negation-affirmation-pivot** (PRIORIDAD 1, blanda-gated) — la familia antítesis: « no es X, es Y ».
- **es-anglicism** (gate fuerte) — calcos del inglés (distinguir por objeto/contexto).
- **lexical-cliche / superficial-analysis / hedge-scaffolding / opener-cliche / structural-connector / rhetorical-padding / copula-avoidance / typography** — equivalentes españoles de las categorías inglesas.

## Anglicismos — distinguir por objeto/contexto

- **soportar** [formato/idioma/archivo] = 'support' (correcto: *soportar* peso/ruido). Gate, contexto técnico.
- **aplicar a/para** = postular (muy arraigado en LatAm; FP alto). Gate fuerte.
- **remover** [archivo/dato/cuenta] = quitar/eliminar (correcto: *remover* = revolver; arraigado en LatAm). Gate fuerte.
- **eventualmente** = finalmente / con el tiempo (correcto: ocasionalmente). Gate.

## Puntuación

- Falta de signos de apertura **¿** **¡** : tell solo en prosa formal y pulida; en registro informal o móvil es normal. Gate.
- **Raya (—) / em dash:** solo densidad alta. La raya de diálogo es CORRECTA (RAE) — no señalar.
- **Tildes faltantes** (*mas* por *más*, *esta* por *está*, *articulo* por *artículo*) en texto por lo demás cuidado: posible artefacto de pipeline con teclado inglés. FP muy alto (usuarios móviles) → gate fuerte + registro formal.

## Frases firma (alto valor)

- **Pivotes:** « no es X, es Y » / « no se trata de X, sino (de) Y » / « no solo X, sino (también) Y » / « más que X, es Y » / « no tanto X como Y ».
- **Aperturas:** « en el mundo acelerado de hoy » / « en la era de » / « en este artículo exploraremos » / « imagina un escenario en el que » / « adentrándonos en ».
- **Andamiaje:** « es importante destacar/señalar que » / « cabe señalar que » / « a la hora de » / « de cara a ».
- **Inflación de significado:** « un testimonio/testamento de » / « desempeña/juega un papel crucial/fundamental » / « punto de inflexión ».
- **Clichés léxicos:** « en el panorama cambiante » / « navegar por el complejo panorama » / « rico tapiz » / « sinfonía de » / « revolucionar » ; intensificadores: *crucial, fundamental, robusto, vibrante, transformador*.
- **Gerundios de cola (superficial-analysis):** « , permitiendo ahorrar tiempo » / « , garantizando… » / « , impulsando… ».
- **Conectores (densidad):** además, asimismo, por otro lado, por consiguiente, en este sentido, en conclusión, en resumen.
- **Equilibrio vacío:** « si bien X, también Y » / « no existe una solución única para todos ».

## Qué hacer

Mismos principios que en inglés: citar la frase exacta, explicar el problema,
proponer un reemplazo que preserve la voz. Para un anglicismo, proponer el
término correcto según el objeto (« aplicar al puesto » → « postular al puesto »).
No sobre-corregir: la prosa española viva conserva su textura.
