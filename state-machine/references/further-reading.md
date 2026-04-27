# Further reading

Curated, annotated links. Read in roughly this order if you are new to state
machines and want depth.

## Foundational

- **David Harel, *Statecharts: A Visual Formalism for Complex Systems*, 1987**
  ([PDF](https://www.state-machine.com/doc/Harel87.pdf), [ScienceDirect](https://www.sciencedirect.com/science/article/pii/0167642387900359))
  The paper that introduced hierarchy, concurrency and history to state
  machines. Dense but the figures are still the clearest in the field. Skim
  sections 1, 2 and 6.
- **statecharts.dev** ([site](https://statecharts.dev/), [glossary](https://statecharts.dev/glossary/))
  The best free conceptual reference. Short pages, well cross-linked. Start
  with *what is a state machine*, *what is a statechart*, and *state explosion*.
- **W3C SCXML 1.0 Recommendation, 2015** ([spec](https://www.w3.org/TR/scxml/))
  The XML standard for executable statecharts. Useful as a precise vocabulary
  reference even if you never write SCXML. Defines internal vs external
  transitions and run-to-completion semantics.
- **Stately.ai docs: *State machines and statecharts*** ([page](https://stately.ai/docs/state-machines-and-statecharts))
  Modern, accessible primer. Maps SCXML / Harel terminology onto XState v5.

## UI / web

- **Kent C. Dodds, *Stop using isLoading booleans*** ([blog](https://kentcdodds.com/blog/stop-using-isloading-booleans))
  The canonical refactor: boolean flags to a `status` enum with discriminated
  union. The example in `anti-patterns.md` is adapted from this post.
- **Kent C. Dodds, *Don't Sync State, Derive It*** ([blog](https://kentcdodds.com/blog/dont-sync-state-derive-it))
  Companion piece. Explains why most state-syncing bugs vanish when you
  derive instead of mirror.
- **David Khourshid, *Infinitely Better UIs with Finite Automata*** ([YouTube](https://www.youtube.com/watch?v=VU1NKX6Qkxc))
  The talk that brought state machines back to mainstream UI work.
  Foundational for XState.
- **David Khourshid, *Crafting Stateful Styles with State Machines*** ([YouTube](https://www.youtube.com/watch?v=0cqeGeC98MA))
  Applies the same thinking to CSS / animation states.
- **Krasimir Tsonev (24ways), *The (Switch)-Case for State Machines in User Interfaces*** ([article](https://24ways.org/2018/state-machines-in-user-interfaces/))
  Short, accessible essay. Good link to send to someone who has never thought
  about UI as a state machine.
- **XState by Example** ([gallery](https://xstatebyexample.com/))
  Recipe gallery. Form, fetch, auth, drag-and-drop, multiplayer.
- **Mastery.games, *State Machines in React*** ([article](https://mastery.games/post/state-machines-in-react/))
  Practical React patterns with and without XState.
- **cassiozen, *useStateMachine* hook** ([GitHub](https://github.com/cassiozen/useStateMachine))
  Sub-1KB React hook. Useful as a worked-out small reference implementation.

## Systems / backend

- **Miro Samek, *Practical UML Statecharts in C/C++*, 2nd ed., 2008**
  ([book site](https://www.state-machine.com/psicc2), [free PDF](https://www.state-machine.com/doc/PSiCC2.pdf))
  The reference for embedded statecharts. Part I teaches HSM concepts and
  coding techniques, Part II is the QP framework reference. Chapter 2 is the
  best concise explanation of run-to-completion and event-driven dispatch.
- **Quantum Leaps QP framework** ([site](https://www.state-machine.com/))
  Open-source HSM / RTEF for C and C++. Powers the QM modeler that generates
  production code from diagrams.
- **Yoshua Wuyts, *State Machines* blog series** ([part 1](https://blog.yoshuawuyts.com/state-machines/), [part 2](https://blog.yoshuawuyts.com/state-machines-2/index), [part 3](https://blog.yoshuawuyts.com/state-machines-3))
  Three-part walkthrough in Rust covering enum-based, typestate and
  hierarchical designs. Ends with a discussion of compile-time checking.
- **Hoverbear, *Pretty State Machine Patterns in Rust*** ([blog](https://hoverbear.org/blog/rust-state-machine-pattern/))
  Practical typestate walkthrough. Pairs well with Wuyts.
- **statig crate** ([GitHub](https://github.com/mdeloof/statig), [docs.rs](https://docs.rs/statig/))
  Rust HSM crate with a `#[state_machine]` macro. Designed for embedded.
- **corrode.dev, *Using Enums to Represent State*** ([blog](https://corrode.dev/blog/enums/))
  Rust enum-state idioms. Short and idiomatic.
- **Baeldung, *Spring State Machine*** ([article](https://www.baeldung.com/spring-state-machine))
  The standard intro for the Spring StateMachine project.
- **Baeldung, *Implementing Simple State Machines with Java Enums*** ([article](https://www.baeldung.com/java-enum-simple-state-machine))
  When you do not need a library: enum + transition-method pattern.
- **squirrel-foundation** ([GitHub](https://github.com/hekailiang/squirrel))
  Java HSM library with fluent builders, method-call actions and optional
  declarative annotations.
- **Spring StateMachine reference docs** ([site](https://docs.spring.io/spring-statemachine/docs/current/reference/))
  Full reference. Covers persistence, distributed machines, security
  integration, region support.

## Standards and tools

- [W3C SCXML 1.0 Recommendation](https://www.w3.org/TR/scxml/)
- [Stately Studio (visual XState editor)](https://stately.ai/studio)
- [Stately Inspector (live debugger)](https://stately.ai/docs/inspector)
- [Mermaid stateDiagram-v2](https://mermaid.js.org/syntax/stateDiagram.html)
- [PlantUML state diagram](https://plantuml.com/state-diagram)
- [QM Modeler (Quantum Leaps)](https://www.state-machine.com/products/qm/)
- [SCION (SCXML interpreter)](https://gitlab.com/scion-scxml/scion)

## Short reading path

1. Read [statecharts.dev/what-is-a-statechart](https://statecharts.dev/what-is-a-statechart.html)
2. Read [statecharts.dev/state-machine-state-explosion](https://statecharts.dev/state-machine-state-explosion.html)
3. Watch [Khourshid's talk](https://www.youtube.com/watch?v=VU1NKX6Qkxc)
4. Read [Kent C. Dodds, *Stop using isLoading booleans*](https://kentcdodds.com/blog/stop-using-isloading-booleans)
5. Skim the implementation file in this skill that matches your stack.
