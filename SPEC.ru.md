# TimeLayer Agent Gate (TL-Gate)

## Полная архитектурная спецификация универсального слоя управления AI-агентами

**Статус:** Proposed Architecture Specification  
**Версия документа:** 0.1  
**Дата:** 11 июля 2026 года  
**Рабочее имя нового инструмента:** `TimeLayer Agent Gate` / `TL-Gate`  
**Предлагаемый отдельный репозиторий:** `timelayer-agent-gate`

> Название является рабочим. Архитектурная граница инструмента от названия не зависит.

---

# 0. Архитектурное решение

Новый инструмент должен быть **независимым от конкретного AI-агента, модели, фреймворка и оркестратора**.

Он не является новым оркестратором и не заменяет существующие системы планирования, swarm-координации, маршрутизации задач или выбора моделей.

Его назначение:

> поставить обязательную, fail-closed, receipt-driven границу между намерением любого агента и реальным побочным эффектом; доказуемо связать разрешение, область действия, выбранный инструмент, фактическое исполнение, проверку результата и финальность; после этого опционально передать доказанный результат в Second Brain.

Каноническая формула:

```text
Orchestrator plans.
Agent proposes.
TL-Gate governs and executes through a controlled boundary.
Verifier checks receipts.
Second Brain stores only receipt-bound knowledge.
```

На русском:

```text
Оркестратор планирует.
Агент предлагает действие.
TL-Gate управляет допуском и проводит действие через контролируемую границу.
Verifier проверяет квитанции.
Second Brain принимает только знание, связанное с доказательством.
```

---

# 1. Исходная проблема

Современный агентный стек обычно умеет:

- разбивать задачу на подзадачи;
- выбирать агентов и модели;
- вызывать инструменты;
- обмениваться сообщениями;
- использовать память;
- повторять удачные траектории;
- выполнять shell, filesystem, browser, MCP, API и database calls.

Но сам факт наличия orchestration layer не доказывает:

1. кто разрешил действие;
2. какому агенту оно было разрешено;
3. в какой области оно было разрешено;
4. какой конкретно инструмент и какая версия были допущены;
5. совпали ли фактические аргументы с разрешёнными;
6. что инструмент действительно выполнил;
7. какой результат реально получен;
8. какой проверяющий проверил результат;
9. прошла ли проверка;
10. какая версия результата является финальной;
11. не изменился ли результат или знание после проверки;
12. можно ли независимо перепроверить всю цепочку без доверия к оркестратору.

Обычные hooks, callbacks, tracing и logs не закрывают эту задачу полностью, потому что они чаще всего:

- находятся внутри той же доверенной среды;
- могут быть отключены или обойдены;
- фиксируют заявление о событии, а не криптографически связанную цепочку;
- не всегда стоят перед реальным побочным эффектом;
- могут быть изменены владельцем журнала;
- не связывают разрешение, точный input, exact tool, output, validation и finality в одну причинную цепь.

---

# 2. Граница нового продукта

## 2.1. Что TL-Gate делает

TL-Gate должен:

- принимать действие от любого внешнего оркестратора;
- переводить его в канонический `ActionIntent`;
- установить identity инициатора и исполняющего агента;
- проверить `permission_receipt`;
- проверить `scope_receipt`;
- проверить `tool_receipt`;
- запретить выполнение при любом конфликте;
- направить разрешённое действие только через Controlled Tool Broker;
- зафиксировать точный input и фактический output;
- сформировать `execution_receipt`;
- запустить назначенные validators;
- сформировать `validation_receipt`;
- сформировать `final_receipt` только для принятого результата;
- сохранить финальный evidence capsule без бесконечного mutable log;
- опционально передать результат в Second Brain;
- автоматически лишать downstream-объекты доверенного статуса при несовпадении digest;
- обеспечивать offline verification через отдельный `timelayer-verifier`.

## 2.2. Что TL-Gate не делает

TL-Gate не должен:

- выбирать бизнес-цель пользователя;
- строить общий план вместо оркестратора;
- быть LLM;
- быть системой vector memory;
- автоматически объявлять содержание истинным;
- заменять `timelayer-verifier`;
- поглощать TL-Agent;
- поглощать Second Brain;
- превращать `receipt-driven-examples` в production runtime;
- хранить содержимое пользователя в TimeLayer network;
- считать текст агента доказательством;
- считать hook доказательством исполнения;
- разрешать агенту выпускать собственные действительные квитанции;
- сохранять бесконечный системный журнал как источник истины.

---

# 3. Отношение к четырём существующим репозиториям

На момент подготовки спецификации публичная организация TimeLayer-OS содержит четыре отдельных репозитория. Новый инструмент должен стать **пятым отдельным репозиторием**, а не механическим объединением существующих.

## 3.1. `timelayer-verifier`

Роль:

- отдельный offline verifier;
- проверяет пару `cert.tlcert + bundle.tlbundle`;
- пересчитывает BLAKE3 commitment;
- проверяет Ed25519 quorum;
- подтверждает `FINAL`;
- возвращает точный verdict `VALID FINAL` либо отказ;
- не доказывает истинность содержания.

Интеграция с TL-Gate:

- внешний бинарник или vendored library через стабильный verifier interface;
- TL-Gate не должен реализовывать альтернативную «упрощённую» проверку;
- любое несовпадение output, exit code или schema означает STOP;
- verifier остаётся независимо заменяемым и собираемым.

## 3.2. `TL-Agent`

Роль:

- отдельный receipt-gated SDK для разрешённых действий агента;
- topology validation;
- permission bundle;
- fail-closed gate;
- текущая ветка `v0.1.x` не является sandbox и пока не исполняет все scope/policy fields принудительно.

Интеграция с TL-Gate:

- опциональный `TL-Agent Bundle Adapter`;
- существующий permission bundle может быть одним из источников `permission_receipt`;
- TL-Gate не зависит от TL-Agent как от единственного источника разрешений;
- TL-Gate добавляет универсальную нормализацию, controlled execution, exact input/output binding, validation и finalization;
- код и репозиторий TL-Agent остаются отдельными.

## 3.3. `timelayer-second-brain`

Роль:

- отдельная база знаний на квитанциях;
- raw source → wiki claim anchors → grounding verdict → trusted state;
- status является вычисляемым свойством;
- изменение текста меняет digest и снимает trusted;
- квитанция доказывает факт проверки конкретного текста, но не абсолютную истинность утверждения.

Интеграция с TL-Gate:

- опциональный `Second Brain Bridge`;
- в мозг передаются только результаты, имеющие допустимый `final_receipt`;
- raw source, claim, grounding verdict и финальный status остаются ответственностью Second Brain;
- TL-Gate не становится knowledge base;
- Second Brain не становится execution broker.

## 3.4. `receipt-driven-examples`

Роль:

- отдельная библиотека примеров receipt-driven/logless pattern;
- демонстрирует правило «нет действительной квитанции — нет действия»;
- является reference/example repository, а не production control plane.

Интеграция с TL-Gate:

- используется как источник простых интеграционных паттернов и test fixtures;
- не является runtime dependency;
- новый production код должен находиться в собственном репозитории.

## 3.5. Итоговая композиция

```text
+---------------------------+
| External Agent            |
| Orchestrator / Harness     |
+-------------+-------------+
              |
              v
+---------------------------+
| NEW: TimeLayer Agent Gate |
| Universal control plane   |
+------+------+-------------+
       |      |      |
       |      |      +--------------------+
       |      |                           |
       v      v                           v
 TL-Agent   timelayer-verifier     Second Brain
 adapter    independent check      optional knowledge sink

receipt-driven-examples = reference patterns/tests, not runtime core
```

---

# 4. Главные принципы

## P-01. Orchestrator neutrality

Core не должен знать внутреннюю модель конкретного оркестратора. Он принимает только канонический intent.

## P-02. No valid receipt → no action

Недействительная, отсутствующая, просроченная, отозванная, конфликтующая или неприменимая квитанция всегда означает STOP.

## P-03. Pre-effect enforcement

Проверка permission, scope и tool binding должна стоять **до реального побочного эффекта**.

## P-04. Exact binding

Разрешение должно быть связано с точным:

- principal;
- agent instance;
- action kind;
- target;
- tool identity;
- arguments digest;
- policy digest;
- causal parent;
- attempt number.

## P-05. Agent cannot self-authorize

Агент, оркестратор и adapter не могут самостоятельно объявить receipt действительным.

## P-06. Model output is not proof

Фразы «готово», «успешно», «проверено» и tool callback не являются доказательством.

## P-07. Validation is explicit

Результат не становится финальным без назначенной проверки.

## P-08. Finality is explicit

Наличие output не равно `FINAL`. До final receipt результат считается `NON_FINAL`.

## P-09. Logless by default

Авторитетное состояние хранится как bounded evidence capsules и immutable receipts, а не как бесконечный изменяемый журнал.

## P-10. BLAKE3 only

В активном каноническом пути нового инструмента используется только BLAKE3 с domain separation. SHA-256 не допускается как canonical digest TL-Gate.

## P-11. LocalPoH orders; wall clock informs

Причинный порядок задаётся local sequence/LocalPoH и digest links. Wall-clock timestamp является информационным полем, а не источником истины.

## P-12. Fail-closed propagation

Неопределённость не превращается в разрешение. `UNKNOWN`, `INCONCLUSIVE`, timeout, verifier unavailable и schema mismatch не проходят дальше.

## P-13. Trust cannot be amplified by delegation

Дочерний агент может получить только тот же или более узкий scope. Расширение полномочий через delegation запрещено.

## P-14. Separate truth domains

- orchestrator отвечает за план;
- TL-Gate отвечает за governed execution;
- validator отвечает за свою проверку;
- TimeLayer receipt отвечает за attestation/finality конкретного digest;
- Second Brain отвечает за вычисляемое состояние знания.

---

# 5. Термины

| Термин | Определение |
|---|---|
| `Principal` | Пользователь, организация или системная роль, от имени которой разрешено действие |
| `Orchestrator` | Внешняя система, планирующая и распределяющая задачи |
| `AgentInstance` | Конкретный исполняющий агент в конкретной сессии |
| `Adapter` | Переводчик протокола внешнего harness в TL-Gate protocol |
| `ActionIntent` | Каноническое неизменяемое описание предлагаемого действия |
| `Capability` | Тип допустимого действия: read, write, execute, network, browser, database и т. п. |
| `Scope` | Точные пределы capability: paths, hosts, methods, namespaces, quotas, time/attempt limits |
| `ToolBinding` | Связь действия с конкретным tool, version, executable/image digest и input schema |
| `Controlled Tool Broker` | Единственная разрешённая точка реального вызова инструментов |
| `Validator` | Механический, модельный или человеческий проверяющий |
| `Evidence Capsule` | Ограниченный immutable пакет доказательств одной action chain |
| `Receipt Chain` | Причинно связанная последовательность квитанций одного действия |
| `Final` | Принятый и нотариально финализированный конкретный результат |
| `Stop-State` | Состояние безопасной остановки, из которого нет автоматического продолжения без разрешённой recovery-процедуры |

---

# 6. Верхнеуровневая схема

```text
Any External Orchestrator / Agent Harness
        |
        | adapter protocol
        v
+------------------------------------------------------+
|                 TimeLayer Agent Gate                 |
|                                                      |
|  1. Adapter Gateway                                  |
|  2. Intent Normalizer                                |
|  3. Identity / Delegation Resolver                   |
|  4. Policy Compiler                                  |
|  5. Pre-Execution Gate                               |
|     - permission_receipt                             |
|     - scope_receipt                                  |
|     - tool_receipt                                   |
|  6. Controlled Tool Broker                          |
|  7. Result Capture                                   |
|  8. Validation Engine                                |
|     - validation_receipt                             |
|  9. Finalizer                                        |
|     - final_receipt                                  |
| 10. Evidence Capsule Store                           |
| 11. Stop-State / Recovery Controller                 |
+------------------------+-----------------------------+
                         |
          +--------------+--------------+
          |                             |
          v                             v
 timelayer-verifier              Second Brain Bridge
 offline verification            optional promotion
```

Полный поток:

```text
External plan
    ↓
Action proposal
    ↓
Canonical ActionIntent
    ↓
permission_receipt
    ↓
scope_receipt
    ↓
tool_receipt
    ↓
ALLOW or STOP
    ↓ ALLOW
Controlled execution
    ↓
execution_receipt
    ↓
validation
    ↓
validation_receipt
    ↓ PASS
finalization
    ↓
final_receipt
    ↓
Evidence capsule
    ↓ optional
Second Brain ingest / claim grounding / trust computation
```

---

# 7. Trust boundaries

## TB-01. External orchestrator is untrusted for proof

Оркестратор может предложить действие и передать metadata, но не может:

- объявить собственное разрешение действительным;
- подменить verifier verdict;
- объявить output финальным;
- повысить knowledge status до trusted.

## TB-02. Adapter is untrusted for semantics

Adapter нормализует вызов, но core повторно проверяет:

- schema;
- digest;
- identity;
- capability mapping;
- target mapping;
- version negotiation.

## TB-03. Tool is untrusted for correctness

Tool output считается данными до validation.

## TB-04. Validator is accountable but not omniscient

Validator подтверждает только выполненную им проверку. Его receipt не превращает ошибочную методику в истину.

## TB-05. Host may be compromised

Обычный local-process mode не защищает от root/admin compromise. Для сильного enforcement нужен isolated broker mode или hardware boundary.

## TB-06. TimeLayer network does not receive user content

На нотариальный слой передаются canonical digests и необходимые metadata, а не raw files, prompts, secrets или private outputs.

---

# 8. Компоненты TL-Gate

## 8.1. Adapter Gateway

Обязанности:

- принимать вызовы от внешних harness;
- поддерживать version handshake;
- выдавать capability manifest;
- переводить вызов в `ActionIntentDraft`;
- запрещать неизвестные обязательные поля;
- сохранять raw adapter payload только как optional user-owned evidence reference.

Основные transport profiles:

- MCP proxy/server;
- local Unix socket;
- Windows named pipe;
- stdio protocol;
- local HTTP loopback;
- Rust SDK;
- generic JSON-RPC adapter.

Публичный сетевой listener по умолчанию запрещён.

## 8.2. Intent Normalizer

Преобразует adapter-specific request в canonical `ActionIntent`.

Нормализатор обязан:

- убрать неоднозначность target;
- разрешить relative path в canonical path;
- нормализовать HTTP host/method/path;
- отделить secret reference от secret value;
- вычислить BLAKE3 digest аргументов;
- определить side-effect class;
- назначить `action_id`, `chain_id`, `attempt`;
- связать действие с parent task/delegation.

После нормализации intent неизменяем. Любое изменение создаёт новый `action_id` и новую цепочку.

## 8.3. Identity and Delegation Resolver

Проверяет:

- principal identity;
- orchestrator identity;
- agent instance identity;
- session binding;
- delegation lineage;
- отсутствие scope amplification;
- revocation state;
- replay protection.

## 8.4. Policy Compiler

Преобразует human/operator policy в детерминированный policy IR.

Policy IR должен поддерживать:

- allow/deny capabilities;
- path selectors;
- hostname/IP selectors;
- HTTP methods;
- database schemas/tables/operations;
- tool allowlist;
- version constraints;
- read/write/execute flags;
- network egress policy;
- maximum payload/result sizes;
- attempt limits;
- validation requirements;
- human approval requirements;
- data classification;
- retention profile;
- execution isolation profile.

Policy text не проверяется во время action path. Проверяется только canonical compiled policy digest.

## 8.5. Receipt Resolver

Получает квитанции из разрешённых источников:

- pre-provisioned offline bundle;
- TimeLayer issue client;
- TL-Agent bundle adapter;
- enterprise policy issuer;
- human authorization workflow.

Resolver не доверяет источнику до проверки через verifier.

## 8.6. Verifier Bridge

Единый интерфейс:

```text
verify(cert_path, bundle_path, expected_subject_digest) ->
    VALID_FINAL | NOT_VALID | UNVERIFIABLE | ERROR
```

Правила:

- только точный `VALID FINAL` и exit code 0 разрешают продолжение;
- проверяется expected content binding;
- неожиданный stdout/stderr или schema version означает STOP;
- verifier version и binary digest включаются в evidence capsule;
- verifier может быть внешним бинарником или встроенной официальной library build, но не альтернативной реализацией.

## 8.7. Pre-Execution Gate

Проверяет обязательную тройку:

1. `permission_receipt`;
2. `scope_receipt`;
3. `tool_receipt`.

Решение только двух типов:

```text
ALLOW
STOP(reason_code)
```

Скрытого «best effort allow» нет.

## 8.8. Controlled Tool Broker

Единственная точка реального side effect.

Поддерживаемые connector classes:

- filesystem;
- shell/process;
- MCP tool;
- HTTP/API;
- browser;
- database;
- message/email/calendar;
- cloud infrastructure;
- custom connector SDK.

Broker обязан:

- повторно проверить digest intent перед execution;
- разрешать только tool binding из verified receipt;
- выдавать tool только разрешённые secret handles;
- блокировать прямой environment inheritance;
- фиксировать exact input bytes/digest;
- фиксировать exact output/effect evidence;
- применять timeout/resource limits;
- не позволять adapter выполнить effect самостоятельно.

## 8.9. Result Capture

Создаёт immutable `ExecutionEvidence`:

- exact input digest;
- tool digest;
- environment profile digest;
- output digest;
- exit status;
- structured side-effect summary;
- created/modified/deleted object digests;
- LocalPoH start/end positions;
- causal parent digest;
- bounded stdout/stderr references;
- connector attestation, если доступна.

## 8.10. Validation Engine

Запускает policy-defined validators:

- deterministic validator;
- schema validator;
- unit/integration tests;
- diff policy validator;
- security scanner;
- independent model judge;
- human reviewer;
- external service verifier.

Verdict:

```text
PASS
FAIL
INCONCLUSIVE
```

Только `PASS` удовлетворяющего threshold policy позволяет finalization.

## 8.11. Finalizer

Формирует final subject digest из:

- ActionIntent digest;
- permission receipt digest;
- scope receipt digest;
- tool receipt digest;
- execution receipt digest;
- validation receipt digest;
- final result digest;
- causal chain position;
- policy digest.

После получения TimeLayer proof создаётся `final_receipt`.

## 8.12. Evidence Capsule Store

Хранит не глобальный log, а отдельные bounded capsules.

Пример:

```text
evidence/
  <chain_id>/
    manifest.tl.json
    intent.tlbin
    receipts/
      permission/
      scope/
      tool/
      execution/
      validation/
      final/
    refs/
      input.ref
      output.ref
      diff.ref
    state/
      final.state
```

Содержимое inputs/outputs может оставаться в user-owned storage. Capsule содержит digest и locator policy, но не обязана содержать raw content.

## 8.13. Stop-State / Recovery Controller

Переводит chain или session в Stop-State при:

- verifier error;
- receipt mismatch;
- scope violation;
- tool substitution;
- replay;
- validation failure;
- finalization conflict;
- state corruption;
- recovery divergence;
- direct bypass detection.

Возобновление возможно только по новой recovery authorization, связанной с остановленной цепочкой.

---

# 9. Канонический жизненный цикл действия

## 9.1. Этап A — Proposal

Оркестратор передаёт proposal:

```json
{
  "orchestrator": "external",
  "session_ref": "session-local-ref",
  "agent_ref": "agent-local-ref",
  "tool": "filesystem.write",
  "target": "./src/main.rs",
  "arguments": {
    "patch_ref": "user-owned://patch/42"
  }
}
```

Proposal не является разрешением.

## 9.2. Этап B — Canonicalization

TL-Gate строит immutable `ActionIntent` и вычисляет:

```text
intent_digest = BLAKE3("TL-GATE/INTENT/v1" || canonical_intent)
```

## 9.3. Этап C — Authorization

Проверяется `permission_receipt`, связанный с:

- principal;
- agent instance;
- capability;
- intent digest или допустимым action template;
- delegation lineage;
- validity/revocation domain.

## 9.4. Этап D — Scope enforcement

Проверяется точное соответствие target и constraints.

Пример:

```text
permission: filesystem.write
scope: /workspace/project-a/src/**
actual target: /workspace/project-a/src/main.rs
result: MATCH
```

Символические ссылки, path traversal и mount escape должны разрешаться до сравнения.

## 9.5. Этап E — Tool binding

Проверяется:

- tool ID;
- semantic version constraint;
- executable/container digest;
- adapter version;
- input schema digest;
- output schema digest;
- allowed environment profile;
- connector profile.

## 9.6. Этап F — Pre-gate decision

```text
all mandatory receipts valid
AND all subjects match
AND no revocation
AND no replay
AND topology/delegation valid
AND policy match
    => ALLOW
otherwise
    => STOP
```

## 9.7. Этап G — Controlled execution

Broker выполняет exact intent.

Любое изменение аргументов после ALLOW создаёт новый intent и требует новой проверки.

## 9.8. Этап H — Execution receipt

Формируется receipt над точным execution evidence.

## 9.9. Этап I — Validation

Validators проверяют exact result digest.

## 9.10. Этап J — Finalization

Только accepted chain получает `final_receipt`.

## 9.11. Этап K — Downstream use

Только final result может:

- быть передан следующему агенту как trusted execution artifact;
- быть использован как dependency другой цепочки;
- быть предложен Second Brain;
- быть экспортирован внешнему аудитору как завершённое доказательство.

---

# 10. Модель квитанций

## 10.1. Общий envelope

Каждая TL-Gate receipt должна содержать canonical поля:

```json
{
  "wire": "TL-GATE-WIRE:v1",
  "receipt_kind": "permission_receipt",
  "receipt_id": "...",
  "chain_id": "...",
  "action_id": "...",
  "attempt": 1,
  "principal_id": "...",
  "agent_instance_id": "...",
  "orchestrator_id": "...",
  "subject_digest": "b3:...",
  "policy_digest": "b3:...",
  "causal_parent_digest": "b3:...",
  "previous_receipt_digest": "b3:...",
  "local_poh_tick": 1042,
  "wall_clock_hint": "2026-07-11T00:00:00Z",
  "nonce": "...",
  "issuer_ref": "...",
  "receipt_digest": "b3:..."
}
```

`wall_clock_hint` не участвует в разрешении как единственный критерий.

## 10.2. Domain separation

Для каждого вида receipt используется отдельный domain:

```text
TL-GATE/PERMISSION/v1
TL-GATE/SCOPE/v1
TL-GATE/TOOL/v1
TL-GATE/EXECUTION/v1
TL-GATE/VALIDATION/v1
TL-GATE/FINAL/v1
TL-GATE/DELEGATION/v1
TL-GATE/REVOCATION/v1
TL-GATE/STOP/v1
TL-GATE/RECOVERY/v1
```

## 10.3. Обязательная цепочка

```text
permission_receipt
    ↓ previous_receipt_digest
scope_receipt
    ↓
tool_receipt
    ↓
execution_receipt
    ↓
validation_receipt
    ↓
final_receipt
```

Нельзя заменить один вид receipt другим.

---

# 11. Семантика обязательных квитанций

## 11.1. `permission_receipt`

Доказывает:

- кто выдал разрешение;
- какому principal/agent;
- на какой capability;
- для какого action template или exact intent;
- с каким delegation parent;
- в каком revocation epoch;
- на какое число попыток;
- с какой обязательной validation policy.

Не доказывает:

- что target входит в scope;
- что выбран правильный tool;
- что действие выполнено;
- что результат корректен.

## 11.2. `scope_receipt`

Доказывает точные пределы действия.

Поля:

```text
capability
resource_namespace
target_selectors
allowed_operations
denied_operations
network_policy
path_policy
data_classification
max_payload
max_result
max_attempts
validity_window/revocation_epoch
human_approval_requirement
```

Scope должен быть machine-enforced, а не только описан текстом.

## 11.3. `tool_receipt`

Доказывает допуск конкретного executor profile.

Поля:

```text
tool_id
tool_version
binary_or_image_digest
connector_id
connector_version
input_schema_digest
output_schema_digest
environment_profile_digest
secret_handle_policy
allowed_endpoints
isolation_profile
```

Изменение бинарника, image, schema или connector требует новой tool receipt.

## 11.4. `execution_receipt`

Доказывает, что зафиксирован конкретный execution evidence package.

Поля:

```text
intent_digest
permission_digest
scope_digest
tool_digest
exact_input_digest
execution_environment_digest
local_poh_start
local_poh_end
exit_status
output_digest
side_effect_digest
connector_attestation_digest
bounded_error_digest
```

Execution receipt не объявляет результат правильным.

## 11.5. `validation_receipt`

Доказывает, что конкретный validator проверил конкретный result digest по конкретной policy.

Поля:

```text
validator_id
validator_type
validator_version_or_model_digest
validation_policy_digest
input_result_digest
evidence_digests
verdict: PASS | FAIL | INCONCLUSIVE
limitations
human_signer_ref (optional)
```

Semantic validator не должен по умолчанию быть тем же agent instance, который создал результат.

## 11.6. `final_receipt`

Доказывает, что конкретная chain version:

- полностью связана;
- прошла требуемый validation threshold;
- принята policy;
- имеет конкретный final result digest;
- получила TimeLayer FINAL attestation.

Поля:

```text
chain_root_digest
intent_digest
permission_digest
scope_digest
tool_digest
execution_digest
validation_digest_set
final_result_digest
final_status
supersedes_digest (optional)
local_poh_final_tick
network_finality_proof_ref
```

Новая версия результата не редактирует старый final receipt. Она создаёт новую цепочку и может указывать `supersedes_digest`.

---

# 12. Дополнительные квитанции

## 12.1. `delegation_receipt`

Нужна для передачи задачи от одного агента другому.

Правило:

```text
child_scope ⊆ parent_scope
child_capabilities ⊆ parent_capabilities
child_expiry ≤ parent_expiry
child_attempts ≤ parent_remaining_attempts
```

Любое расширение означает STOP.

## 12.2. `revocation_receipt`

Отзывает permission, scope, tool или delegation lineage.

Offline bundle должен содержать актуальный revocation epoch или bounded freshness policy. При невозможности определить актуальность применяется configured fail-closed mode.

## 12.3. `stop_receipt`

Фиксирует причину Stop-State, последний валидный digest и запрещённый transition.

## 12.4. `recovery_receipt`

Разрешает строго определённое восстановление:

- retry same intent;
- retry with new tool;
- rollback staged effect;
- abandon chain;
- start superseding chain.

Recovery receipt не может переписать прежние evidence capsules.

---

# 13. Классы действий и исполнение

## 13.1. Class R0 — Pure computation

Нет внешнего side effect.

Примеры:

- локальное вычисление;
- parsing;
- deterministic transform.

Минимальный профиль:

- permission;
- scope;
- tool;
- execution;
- validation;
- final.

## 13.2. Class R1 — Read-only external access

Примеры:

- чтение файла;
- GET request;
- database SELECT.

Дополнительные требования:

- source digest;
- content size limits;
- data classification;
- prompt-injection guard для untrusted text.

## 13.3. Class W1 — Reversible write

Примеры:

- изменение файла в staging branch;
- создание draft;
- запись во временную таблицу.

Требования:

- staged execution предпочтителен;
- diff capture;
- deterministic validation;
- rollback path;
- finalization до promotion/merge.

## 13.4. Class W2 — Transactional external effect

Примеры:

- database transaction;
- infrastructure plan/apply с provider transaction;
- message draft + send.

Требования:

- prepare/commit connector profile;
- exact prepared artifact digest;
- validation перед commit, если tool поддерживает;
- commit evidence;
- final receipt после confirmed commit.

## 13.5. Class W3 — Irreversible or high-impact effect

Примеры:

- отправка денег;
- удаление без rollback;
- публичная публикация;
- production destructive operation;
- отправка юридически значимого сообщения.

Требования:

- exact-intent permission, не широкий template;
- human approval по policy;
- one-shot nonce;
- strict tool binding;
- no retry without new receipt;
- explicit warning that post-execution finality cannot отменить уже совершённый effect.

---

# 14. Режимы enforcement

## 14.1. Cooperative mode

Интеграция через hooks/SDK/CLI.

Плюсы:

- простая интеграция;
- низкий порог входа.

Ограничение:

- агент или host process с прямым доступом к инструментам может обойти gate.

Маркировка evidence:

```text
enforcement_level = cooperative
```

## 14.2. Broker-enforced mode

Агенту доступны только proxy tools TL-Gate.

Меры:

- отдельный process identity;
- filesystem ACL;
- network egress через proxy;
- shell доступ только через broker;
- secrets только через handles;
- container/sandbox profile;
- direct tool binaries недоступны агенту.

Маркировка:

```text
enforcement_level = broker_enforced
```

## 14.3. Isolated mode

Агент работает в контейнере/VM/OS sandbox, а broker находится за отдельной privilege boundary.

Маркировка:

```text
enforcement_level = isolated
```

## 14.4. Air-gapped permission mode

Permission receipts находятся на read-only media; outputs/evidence — на отдельном user-owned writable media.

Маркировка:

```text
enforcement_level = air_gapped_permissions
```

## 14.5. Hardware-backed mode

Будущий профиль:

- hardware key custody;
- TEE/TPM attestation;
- measured broker binary;
- external signing device.

Не является обязательным для v0.1.

---

# 15. Универсальный adapter protocol

## 15.1. Требование

Любой оркестратор подключается через adapter, но core protocol остаётся одинаковым.

## 15.2. Handshake

```json
{
  "method": "gate.hello",
  "protocol": "TL-GATE-ADAPTER:v1",
  "adapter_id": "...",
  "adapter_version": "...",
  "orchestrator_id": "...",
  "capabilities": [
    "tool_call_intercept",
    "stream_result",
    "delegation",
    "session_identity"
  ]
}
```

Ответ:

```json
{
  "accepted_protocol": "TL-GATE-ADAPTER:v1",
  "required_features": [
    "exact_arguments",
    "stable_agent_identity"
  ],
  "gate_mode": "broker_enforced",
  "tool_namespace": "timelayer/*"
}
```

## 15.3. Submit intent

```text
gate.submit_intent(ActionIntentDraft) -> IntentAccepted | IntentRejected
```

## 15.4. Gate decision

```text
gate.authorize(action_id) -> ALLOW | STOP
```

## 15.5. Execute

```text
gate.execute(action_id) -> ExecutionPending | ExecutionCompleted | StopState
```

Оркестратор не получает raw tool handle для обхода broker.

## 15.6. Validate

```text
gate.validate(action_id) -> PASS | FAIL | INCONCLUSIVE
```

## 15.7. Finalize

```text
gate.finalize(action_id) -> FINAL | NON_FINAL | STOP
```

## 15.8. Export evidence

```text
gate.export_capsule(action_id) -> portable evidence capsule
```

---

# 16. MCP-профиль

MCP является удобным универсальным transport, но не источником доверия.

TL-Gate может публиковать один MCP server:

```text
timelayer.discover_tools
timelayer.propose_action
timelayer.execute_action
timelayer.get_status
timelayer.get_result
timelayer.export_receipts
```

Реальные внешние MCP tools подключаются за TL-Gate как upstream servers:

```text
Agent/Orchestrator
       ↓ MCP
TL-Gate MCP Proxy
       ↓ controlled MCP
Upstream Tool Server
```

Прямой доступ агента к upstream MCP endpoint в broker-enforced mode запрещён.

---

# 17. Политика validation

## 17.1. Validator types

### Mechanical

- schema validation;
- exact value/quote matching;
- checksum/digest;
- compiler/test exit;
- diff policy;
- deterministic invariant.

### Semantic

- independent model judge;
- domain-specific classifier;
- contradiction checker.

### Human

- signed manual approval;
- dual control;
- threshold approval.

### External authoritative

- independent API response;
- hardware attestation;
- third-party signature.

## 17.2. Threshold policy

Пример:

```json
{
  "required": [
    {"type": "schema", "count": 1},
    {"type": "tests", "count": 1},
    {"type": "semantic_judge", "count": 1}
  ],
  "deny_on": ["FAIL", "INCONCLUSIVE"],
  "self_validation_allowed": false
}
```

## 17.3. Validator independence

По умолчанию запрещается считать единственным semantic validator тот же model instance, который создал результат.

## 17.4. Validation of side effects

Для tool, который может менять внешний мир, validation должна проверять не только returned text, но и effect evidence:

- final filesystem digest;
- database transaction ID/state;
- remote object version;
- message provider ID;
- API response signature;
- infrastructure state digest.

Если connector не умеет подтвердить effect, receipt должен содержать limitation и более низкий assurance level.

---

# 18. Интеграция с Second Brain

## 18.1. Основное правило

```text
NO final_receipt → NO trusted promotion
```

## 18.2. Поток

```text
TL-Gate final result
    ↓
Second Brain raw source ingest
    ↓
raw_source_receipt
    ↓
claim extraction
    ↓
claim → source fragment anchors
    ↓
grounding validation
    ↓
grounding verdict receipt
    ↓
computed status
```

## 18.3. Mapping status

### `trusted`

Требуется:

- valid final receipt;
- exact content binding;
- source digest match;
- required mechanical checks;
- required semantic/human validation;
- no later edit;
- no revocation/supersession conflict.

### `trusted-mechanical`

Допустимо, когда:

- cryptographic/mechanical binding проходит;
- semantic guarantee отсутствует или validator class неполон;
- ограничения явно отображены.

### `unverified`

Применяется, когда:

- final receipt отсутствует;
- validation incomplete;
- verifier unavailable;
- source changed;
- claim anchor invalid;
- receipt binding mismatch.

## 18.4. Auto-invalidation

Любое изменение:

- текста;
- source fragment;
- source version;
- claim anchor;
- verdict;
- validator set;
- policy;

изменяет digest. Старый trusted status автоматически перестаёт соответствовать текущей версии.

## 18.5. Ограничение

Final execution receipt не доказывает истинность знания. Он доказывает governed execution и конкретный результат. Second Brain обязан отдельно выполнять grounding.

---

# 19. Logless / receipt-driven storage model

## 19.1. Что запрещено

- бесконечный `execution.log` как источник истины;
- mutable audit database, которую оператор может переписать;
- reliance на observability platform для доказательства;
- хранение всей истории prompts/tool outputs без TTL;
- hidden success без final receipt.

## 19.2. Что хранится

### Immutable final evidence

- final receipt pair;
- canonical chain manifest;
- digests обязательных receipts;
- policy/tool/verifier version digests;
- content locators, если разрешены;
- minimal effect evidence.

### Bounded non-final state

- только незавершённые chains;
- TTL;
- maximum count/size;
- cleanup после FINAL/STOP;
- recovery capsule вместо event log.

### Optional debug telemetry

- выключена по умолчанию;
- не является авторитетной;
- имеет TTL;
- не влияет на verdict.

## 19.3. Retention profiles

```text
strict
  keep final receipts + minimal manifests only

minimal
  keep final receipts + bounded validation evidence refs

debug
  keep temporary diagnostic material under explicit TTL
```

## 19.4. Recovery without replay log

После restart TL-Gate сканирует:

- immutable final capsules;
- bounded active state capsules;
- last LocalPoH checkpoint;
- pending tool connector state.

Затем:

```text
Exec(current chain state) == Replay(capsule-derived state)
```

При divergence → Stop-State.

---

# 20. State machine

```text
RECEIVED
  ↓
NORMALIZED
  ↓
AUTHORIZING
  ├─ invalid → STOPPED
  ↓
AUTHORIZED
  ↓
SCOPING
  ├─ mismatch → STOPPED
  ↓
SCOPED
  ↓
TOOL_BINDING
  ├─ mismatch → STOPPED
  ↓
READY
  ↓
EXECUTING
  ├─ connector error → EXECUTION_FAILED
  ├─ uncertain effect → EFFECT_UNKNOWN → STOPPED
  ↓
EXECUTED
  ↓
VALIDATING
  ├─ FAIL → REJECTED
  ├─ INCONCLUSIVE → QUARANTINED
  ↓
VALIDATED
  ↓
FINALIZING
  ├─ no quorum / verifier error → EXECUTED_UNFINALIZED
  ↓
FINAL
  ↓
OPTIONAL_BRAIN_PROMOTION
```

## 20.1. `EXECUTED_UNFINALIZED`

Это обязательное честное состояние.

Оно означает:

- действие уже произошло;
- результат зафиксирован локально;
- final receipt не получен;
- downstream use заблокирован;
- автоматическое объявление success запрещено;
- требуется retry finalization или recovery decision.

## 20.2. `EFFECT_UNKNOWN`

Connector не смог определить, произошло ли внешнее действие.

Повтор запрещён до reconciliation, иначе возможен duplicate effect.

---

# 21. Error model

Минимальные reason codes:

```text
E_RECEIPT_MISSING
E_RECEIPT_INVALID
E_RECEIPT_UNVERIFIABLE
E_RECEIPT_SUBJECT_MISMATCH
E_RECEIPT_EXPIRED
E_RECEIPT_REVOKED
E_INTENT_CHANGED
E_SCOPE_VIOLATION
E_TOOL_NOT_ALLOWED
E_TOOL_DIGEST_MISMATCH
E_SCHEMA_MISMATCH
E_DELEGATION_AMPLIFICATION
E_REPLAY_DETECTED
E_VERIFIER_MISSING
E_VERIFIER_UNEXPECTED_OUTPUT
E_CONNECTOR_FAILURE
E_EFFECT_UNKNOWN
E_VALIDATION_FAILED
E_VALIDATION_INCONCLUSIVE
E_FINALITY_UNAVAILABLE
E_STATE_DIVERGENCE
E_DIRECT_BYPASS_DETECTED
E_RECOVERY_NOT_AUTHORIZED
```

Любая неизвестная ошибка нормализуется в:

```text
E_UNKNOWN_FAIL_CLOSED
```

---

# 22. Security model

## 22.1. Основные угрозы

| Угроза | Контроль |
|---|---|
| Агент сам разрешает себе действие | Независимая квитанция + verifier |
| Подмена args после authorization | Exact intent/arguments digest |
| Подмена tool binary | Tool receipt + binary/image digest |
| Scope escape | Canonical target resolution + machine policy |
| Agent-to-agent privilege escalation | Monotonic delegation narrowing |
| Replay receipt | Nonce, action_id, attempt, LocalPoH, causal chain |
| Ложное «готово» | Execution evidence + validation + final receipt |
| Подмена output | Output digest binding |
| Изменение результата после проверки | Digest mismatch, new chain required |
| Validator self-confirmation | Independent validator policy |
| Prompt injection from source | Untrusted-data guard profile |
| Обход через прямой shell/network | Broker-enforced/isolated mode |
| Переписывание mutable log | Immutable evidence capsules |
| Compromised single operator key | Quorum verification |
| Network unavailable | Pre-issued receipts or fail-closed; existing receipts verify offline |
| Host root compromise | Honest limitation; isolated/hardware profiles |

## 22.2. Secret handling

Receipts и manifests не должны содержать secret values.

Используется:

```text
secret_handle = vault://provider/key-id
```

Broker получает secret только после ALLOW и передаёт его tool через минимально необходимый channel.

Secret value не включается в digest напрямую. Включается policy-bound handle identity и, при необходимости, keyed commitment, который нельзя использовать для восстановления секрета.

## 22.3. Prompt injection

Для untrusted source ingestion применяется отдельный guard profile:

- data marking;
- direct-read prohibition;
- invisible content exposure;
- phrase/classifier scan;
- human acknowledgement для suspect source;
- no writes to protected receipts/raw areas;
- source content никогда не интерпретируется как policy.

## 22.4. Honest limitations

TL-Gate не может гарантировать:

1. истинность утверждения только на основании receipt;
2. отсутствие обхода в cooperative mode;
3. защиту от полностью скомпрометированного root/admin host;
4. отмену необратимого side effect;
5. правильность внешнего tool, который ложно сообщает о результате;
6. semantic correctness без достаточного validator set;
7. актуальность offline revocation state без установленной freshness policy.

Эти ограничения должны отображаться в evidence assurance profile.

---

# 23. Assurance profile

Каждый final capsule содержит:

```json
{
  "enforcement_level": "broker_enforced",
  "authorization_assurance": "network_quorum",
  "tool_identity_assurance": "binary_digest",
  "effect_assurance": "connector_confirmed",
  "validation_assurance": "mechanical_plus_independent_semantic",
  "finality_assurance": "valid_final",
  "host_assurance": "standard_os",
  "limitations": []
}
```

Это предотвращает ложное равенство между:

- hook-only execution;
- broker-enforced execution;
- isolated execution;
- hardware-attested execution.

---

# 24. Wire format

## 24.1. Канонический формат

Нормативный формат: `TL-GATE-WIRE:v1`.

Требования:

- explicit length-prefixed fields;
- fixed field order per schema version;
- UTF-8 normalization rules;
- no floating-point values in signed canonical path;
- integers encoded unambiguously;
- arrays preserve order where order is semantic;
- maps are schema-defined, not arbitrary order;
- unknown required fields reject;
- optional extension fields domain-separated;
- BLAKE3 only.

## 24.2. Human-readable mirror

JSON/YAML может использоваться для UI и debugging, но не является canonical signed representation, если не прошёл официальный canonical encoder.

## 24.3. Digest notation

```text
b3:<64 lowercase hex chars>
```

## 24.4. Version negotiation

Новый major wire version не должен молча приниматься старым core.

```text
unsupported major → STOP
unknown optional extension → ignore only if schema marks it non-critical
unknown critical extension → STOP
```

---

# 25. CLI

Предлагаемый binary:

```text
tl-gate
```

Команды:

```text
tl-gate init <workspace>
tl-gate serve --socket <path>
tl-gate adapters list
tl-gate adapters inspect <id>
tl-gate tools list
tl-gate policy compile <policy>
tl-gate intent submit <file>
tl-gate check <action_id>
tl-gate execute <action_id>
tl-gate validate <action_id>
tl-gate finalize <action_id>
tl-gate status <action_id>
tl-gate stop <action_id>
tl-gate recover <action_id> --receipt <path>
tl-gate verify <capsule>
tl-gate export <action_id> --out <path>
tl-gate audit <workspace>
tl-gate gc --profile strict|minimal|debug
```

Exit codes:

```text
0  success / ALLOW / FINAL
1  STOP / NOT VALID / validation failure
2  malformed input
3  unavailable dependency
4  EXECUTED_UNFINALIZED
5  EFFECT_UNKNOWN
```

---

# 26. Предлагаемая структура нового репозитория

```text
timelayer-agent-gate/
  Cargo.toml
  README.md
  README.ru.md
  SPEC.md
  SPEC.ru.md
  SECURITY.md
  LICENSE

  crates/
    tl-gate-core/
    tl-gate-wire/
    tl-gate-identities/
    tl-gate-policy/
    tl-gate-receipts/
    tl-gate-verifier-bridge/
    tl-gate-broker/
    tl-gate-connectors/
    tl-gate-validation/
    tl-gate-finalizer/
    tl-gate-store/
    tl-gate-recovery/
    tl-gate-adapter-sdk/
    tl-gate-mcp/
    tl-gate-cli/

  connectors/
    filesystem/
    process/
    http/
    mcp/
    database/
    browser/

  adapters/
    generic-jsonrpc/
    generic-mcp/
    generic-cli/
    tl-agent-bundle/

  schemas/
    TL-GATE-WIRE-v1.md
    action-intent-v1.json
    permission-receipt-v1.json
    scope-receipt-v1.json
    tool-receipt-v1.json
    execution-receipt-v1.json
    validation-receipt-v1.json
    final-receipt-v1.json

  policies/
    examples/

  testvectors/
    valid/
    forged/
    replay/
    scope-escape/
    tool-substitution/
    output-substitution/
    validation-fail/
    finality-conflict/

  examples/
    any-orchestrator-mcp/
    local-cli-agent/
    delegated-agent-chain/
    second-brain-promotion/

  docs/
    architecture/
    threat-model/
    adapter-guide/
    connector-guide/
    validator-guide/
    migration-from-hooks/
```

Рекомендуемый основной язык: Rust.

Обоснование:

- существующий verifier и TL-Agent уже используют Rust;
- статический кроссплатформенный binary;
- предсказуемый memory/resource control;
- удобен для local daemon, CLI, proxy и connector runtime;
- снижает количество runtime dependencies.

Adapter SDK может дополнительно иметь bindings для Python, TypeScript и Go, но canonical core остаётся один.

---

# 27. Совместимость с существующими компонентами

## 27.1. Verifier compatibility

TL-Gate обязан поддерживать текущий `cert.tlcert + bundle.tlbundle` verification flow через официальный verifier.

## 27.2. BLAKE3 migration rule

Новый active path использует BLAKE3.

Если legacy component содержит SHA-256-oriented metadata:

- оно может быть прочитано как legacy external field;
- оно не становится canonical TL-Gate digest;
- TL-Gate строит новый BLAKE3-bound subject;
- новый receipt ссылается на legacy artifact как на input, а не продолжает SHA-256 chain.

## 27.3. TL-Agent bundles

Adapter преобразует существующие action permission envelopes в TL-Gate authorization subject, но:

- не расширяет declared scope;
- не считает неисполняемые policy fields уже enforced;
- маркирует assurance level честно;
- требует отдельный scope enforcement в TL-Gate.

## 27.4. Second Brain

Bridge экспортирует:

- final result digest;
- source reference;
- final receipt reference;
- validator summary;
- assurance profile;
- limitations.

Second Brain самостоятельно создаёт свои source/grounding receipts.

---

# 28. Тестовая стратегия

## 28.1. Unit tests

Обязательные области:

- canonical serialization;
- BLAKE3 domain separation;
- exact digest reproducibility;
- path normalization;
- scope subset logic;
- delegation narrowing;
- tool binding;
- state transitions;
- cleanup/TTL;
- recovery reconstruction.

## 28.2. Cross-language vectors

Canonical vectors должны проходить одинаково в:

- Rust;
- Python reference decoder;
- TypeScript reference decoder;
- Go reference decoder.

## 28.3. Negative test vectors

Минимум:

1. missing permission receipt;
2. forged certificate;
3. cert/bundle transplant;
4. changed action arguments;
5. path traversal;
6. symlink escape;
7. host wildcard mismatch;
8. tool binary substitution;
9. schema substitution;
10. replayed one-shot receipt;
11. delegation amplification;
12. output transplant;
13. validator result for another digest;
14. final receipt for another chain;
15. direct broker bypass;
16. crash between effect and finality;
17. duplicate retry after `EFFECT_UNKNOWN`;
18. corrupted active state capsule;
19. old trusted brain page after edit;
20. verifier unexpected stdout.

## 28.4. Integration tests

- generic MCP orchestrator → filesystem broker;
- generic JSON-RPC orchestrator → HTTP broker;
- multi-agent delegation chain;
- read-only offline bundle;
- online issue + offline verify;
- Second Brain promotion and auto-invalidation;
- restart/recovery without mutable history log.

## 28.5. Security tests

- prompt injection corpus;
- argument smuggling;
- Unicode path confusion;
- command escaping;
- environment secret leakage;
- connector privilege escape;
- replay/race conditions;
- TOCTOU between check and execution;
- receipt directory tampering;
- validator substitution.

---

# 29. Минимальные критерии готовности v0.1

v0.1 не считается готовой, пока не выполнены все условия:

## Core

- canonical `ActionIntent` реализован;
- BLAKE3-only canonical path;
- шесть обязательных receipt schemas;
- причинная linkage всех receipts;
- deterministic state machine;
- exact error codes;
- Stop-State.

## Verification

- официальный verifier bridge;
- exact `VALID FINAL` contract;
- valid/forged/transplant test vectors;
- verifier missing/unexpected output fail-closed.

## Enforcement

- минимум broker-enforced filesystem connector;
- минимум broker-enforced process connector;
- generic MCP proxy;
- scope machine enforcement;
- tool binary/schema digest enforcement;
- no argument mutation after ALLOW.

## Validation

- deterministic validator interface;
- external semantic/human validator interface;
- threshold policy;
- PASS/FAIL/INCONCLUSIVE handling.

## Storage

- evidence capsules;
- no mandatory growing log;
- TTL cleanup;
- restart reconstruction;
- `EXECUTED_UNFINALIZED` recovery.

## Integration

- generic orchestrator adapter;
- TL-Agent bundle adapter;
- Second Brain bridge;
- one complete example from proposal to final receipt.

## Documentation

- Russian and English README;
- Russian and English SPEC;
- threat model;
- honest limitations;
- adapter guide;
- connector guide;
- validator guide.

---

# 30. Этапы реализации

## Phase 0 — Protocol freeze

Результат:

- `TL-GATE-WIRE:v1`;
- six receipt schemas;
- state machine;
- error model;
- threat model;
- test vectors.

Нельзя начинать широкую adapter разработку до freeze canonical protocol.

## Phase 1 — Local universal gate

Результат:

- daemon + CLI;
- generic JSON-RPC/stdio adapter;
- filesystem/process broker;
- verifier bridge;
- local evidence capsules;
- deterministic validators.

## Phase 2 — MCP control plane

Результат:

- MCP proxy;
- upstream tool registry;
- tool receipt binding;
- broker-enforced tool namespace;
- no direct upstream access profile.

## Phase 3 — Multi-agent delegation

Результат:

- delegation receipts;
- scope narrowing;
- parent/child causal tree;
- cross-agent final artifact handoff.

## Phase 4 — Second Brain bridge

Результат:

- final artifact export;
- raw source receipt handoff;
- grounding verdict mapping;
- trusted/trusted-mechanical/unverified computation;
- edit invalidation test.

## Phase 5 — Strong isolation

Результат:

- container/VM profiles;
- network egress enforcement;
- secret broker;
- host attestation extensions;
- high-impact action profile.

---

# 31. Обоснование основных решений

## 31.1. Почему отдельный пятый репозиторий

Потому что существующие четыре репозитория имеют разные предметные границы:

- verifier проверяет proof;
- TL-Agent управляет permission bundle конкретного agent SDK;
- Second Brain управляет доказуемым состоянием knowledge base;
- receipt-driven-examples показывает pattern.

Универсальный execution governance runtime является самостоятельным продуктом с собственными:

- protocol;
- daemon;
- broker;
- adapters;
- connectors;
- validators;
- state machine;
- security model.

Смешивание этих ролей создаст циклические зависимости и разрушит независимую проверяемость.

## 31.2. Почему TL-Gate не должен быть оркестратором

Потому что orchestration frameworks быстро меняются и используют разные модели задач. Если core станет зависеть от одного planner/swarm model, он перестанет быть универсальной границей.

Нормализованный intent является устойчивой точкой интеграции:

```text
many orchestrators → one action protocol → one governed execution boundary
```

## 31.3. Почему pre-gate и post-gate разделены

До execution можно доказать только:

- разрешение;
- scope;
- tool binding.

После execution можно доказать:

- фактический input/output;
- effect evidence;
- validation;
- finality.

Один «общий receipt» до действия не может содержать фактический результат. Один receipt после действия не может предотвратить запрещённый side effect.

## 31.4. Почему нужен Controlled Tool Broker

Hook, prompt или SDK-check не гарантируют, что агент не вызовет tool другим путём. Реальный enforcement появляется только тогда, когда side effect проходит через отдельную контролируемую точку.

## 31.5. Почему permission, scope и tool разделены

Они отвечают на разные вопросы:

- permission: можно ли этому субъекту выполнять capability;
- scope: где и в каких пределах;
- tool: чем именно.

Объединение создаёт крупные, плохо переиспользуемые разрешения и усложняет revocation.

## 31.6. Почему execution и validation разделены

Execution receipt фиксирует событие и результат. Validation receipt фиксирует оценку результата. Tool не должен сам объявлять собственный output корректным.

## 31.7. Почему final receipt отдельный

PASS одного validator ещё не означает, что выполнены все policy thresholds и chain complete. Final receipt фиксирует accepted terminal state конкретной версии.

## 31.8. Почему Second Brain подключается после finality

Knowledge base не должна принимать промежуточный, неподтверждённый или изменяемый agent output как trusted source. При этом даже final execution artifact обязан пройти отдельное grounding в Second Brain.

## 31.9. Почему logless

Цель — не удалить всякую диагностику, а убрать зависимость доверия от бесконечного mutable журнала. Авторитетным объектом становится переносимый receipt-bound capsule конкретного действия.

## 31.10. Почему BLAKE3 only

Это обеспечивает один canonical digest domain для нового инструмента и совместимость с текущим TimeLayer verifier commitment model. Legacy SHA-256 metadata не должно проникать в новую причинную цепь как основной hash.

---

# 32. Итоговая продуктовая формула

```text
                    ANY ORCHESTRATOR
                           |
                           v
                 CANONICAL ACTION INTENT
                           |
                           v
                 TIMELAYER AGENT GATE
          +----------------+----------------+
          |                |                |
          v                v                v
   PERMISSION         EXACT SCOPE       EXACT TOOL
          \                |                /
           +---------------+---------------+
                           |
                           v
                CONTROLLED EXECUTION
                           |
                           v
                  EXECUTION RECEIPT
                           |
                           v
                     VALIDATION
                           |
                           v
                  VALIDATION RECEIPT
                           |
                           v
                    FINAL RECEIPT
                           |
              +------------+------------+
              |                         |
              v                         v
      PORTABLE EVIDENCE          SECOND BRAIN
          CAPSULE            claim grounding and
                             computed trust status
```

Главный инвариант:

> Любой оркестратор может предложить действие, но ни один оркестратор, агент, adapter или tool не может самостоятельно превратить своё заявление в разрешённое, проверенное и финальное действие.

Финальная граница ответственности:

```text
Orchestrator = planning and coordination
TL-Gate = authorization, scope, tool control, execution boundary, validation flow, finalization
TL-Agent = separate permission-oriented agent SDK and optional bundle source
Verifier = independent cryptographic/offline receipt verification
Second Brain = separate receipt-bound knowledge system
Receipt-driven examples = separate reference implementation of the pattern
```

---

# 33. Публичные исходные материалы, использованные как baseline

- TimeLayer-OS GitHub organization: https://github.com/TimeLayer-OS
- TL-Agent: https://github.com/TimeLayer-OS/TL-Agent
- TimeLayer Verifier: https://github.com/TimeLayer-OS/timelayer-verifier
- TimeLayer Second Brain: https://github.com/TimeLayer-OS/timelayer-second-brain
- Receipt-driven examples: https://github.com/TimeLayer-OS/receipt-driven-examples
- TimeLayer OS: https://timelayer-os.com/ru/

