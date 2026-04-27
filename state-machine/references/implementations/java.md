# Java: state machine implementations

Four idioms, in order of increasing infrastructure. Pick the smallest one
that solves your problem.

## Approach 1: enum FSM with a transition method

The most idiomatic Java FSM for small machines. Each enum constant
overrides a `next(Event)` method where it has non-default transitions.
This version uses switch expressions, so it requires Java 14+.

```java
public enum OrderState {
    DRAFT {
        @Override public OrderState next(OrderEvent e) {
            return switch (e) {
                case PLACE   -> PLACED;
                case CANCEL  -> CANCELLED;
                default      -> this;
            };
        }
    },
    PLACED {
        @Override public OrderState next(OrderEvent e) {
            return switch (e) {
                case PAY     -> PAID;
                case CANCEL  -> CANCELLED;
                default      -> this;
            };
        }
    },
    PAID {
        @Override public OrderState next(OrderEvent e) {
            return switch (e) {
                case SHIP    -> SHIPPED;
                case REFUND  -> REFUNDED;
                default      -> this;
            };
        }
    },
    SHIPPED {
        @Override public OrderState next(OrderEvent e) {
            return switch (e) {
                case DELIVER -> DELIVERED;
                default      -> this;
            };
        }
    },
    DELIVERED, CANCELLED, REFUNDED;

    public OrderState next(OrderEvent e) { return this; }   // default: no-op
}

enum OrderEvent {
    PLACE, PAY, SHIP, DELIVER, CANCEL, REFUND;
}

class Order {
    private OrderState state = OrderState.DRAFT;
    public void on(OrderEvent e) { state = state.next(e); }
    public OrderState state()    { return state; }
}
```

Pros: zero dependencies, type-safe, one file. The default no-op `next` keeps
terminal states (`DELIVERED`, `CANCELLED`, `REFUNDED`) compact.
Cons: side effects must live outside the enum (the enum should stay pure).
Because `default -> this` silently ignores unsupported events, cover the
transition matrix in tests when ignored events matter.

Reference: [Baeldung, *Implementing Simple State Machines with Java Enums*](https://www.baeldung.com/java-enum-simple-state-machine).

## Approach 2: GoF State pattern

When each state has rich behavior beyond transitions (different rendering,
different validation rules), promote each state to its own class
implementing a common interface.

```java
import java.time.Instant;

interface OrderLifecycleState {
    OrderLifecycleState place(Order o);
    OrderLifecycleState pay(Order o);
    OrderLifecycleState cancel(Order o);
    String render();
}

class Draft implements OrderLifecycleState {
    @Override public OrderLifecycleState place(Order o)  { o.timestamp = Instant.now(); return new Placed(); }
    @Override public OrderLifecycleState pay(Order o)    { return this; }              // ignored
    @Override public OrderLifecycleState cancel(Order o) { return new Cancelled(); }
    @Override public String render() { return "Order draft"; }
}

class Placed implements OrderLifecycleState {
    @Override public OrderLifecycleState place(Order o)  { return this; }
    @Override public OrderLifecycleState pay(Order o)    { return new Paid(); }
    @Override public OrderLifecycleState cancel(Order o) { return new Cancelled(); }
    @Override public String render() { return "Order placed"; }
}

class Paid implements OrderLifecycleState {
    @Override public OrderLifecycleState place(Order o)  { return this; }
    @Override public OrderLifecycleState pay(Order o)    { return this; }
    @Override public OrderLifecycleState cancel(Order o) { return this; }
    @Override public String render() { return "Order paid"; }
}

class Cancelled implements OrderLifecycleState {
    @Override public OrderLifecycleState place(Order o)  { return this; }
    @Override public OrderLifecycleState pay(Order o)    { return this; }
    @Override public OrderLifecycleState cancel(Order o) { return this; }
    @Override public String render() { return "Order cancelled"; }
}

class Order {
    Instant timestamp;
    OrderLifecycleState state = new Draft();
    public void place()  { state = state.place(this); }
    public void pay()    { state = state.pay(this); }
    public void cancel() { state = state.cancel(this); }
    public String render() { return state.render(); }
}
```

Pros: state-specific data and behavior live together. The "Open / Closed"
principle applies: add a state without touching the others.
Cons: heavier than the enum form; one class per state.

## Approach 3: Spring StateMachine

For workflow engines, persistent processes, and machines that span requests,
use [Spring StateMachine](https://spring.io/projects/spring-statemachine/).
Supports hierarchy, parallel regions, history, persistence, distributed
machines, and Spring Security integration.

```java
// Sketch; imports and version-specific Spring StateMachine base classes omitted.
@Configuration
@EnableStateMachine
public class OrderStateMachineConfig
        extends EnumStateMachineConfigurerAdapter<OrderState, OrderEvent> {

    @Override
    public void configure(StateMachineStateConfigurer<OrderState, OrderEvent> states)
            throws Exception {
        states.withStates()
              .initial(OrderState.DRAFT)
              .end(OrderState.DELIVERED)
              .end(OrderState.CANCELLED)
              .states(EnumSet.allOf(OrderState.class));
    }

    @Override
    public void configure(StateMachineTransitionConfigurer<OrderState, OrderEvent> trans)
            throws Exception {
        trans.withExternal()
                .source(OrderState.DRAFT).target(OrderState.PLACED).event(OrderEvent.PLACE)
              .and().withExternal()
                .source(OrderState.PLACED).target(OrderState.PAID).event(OrderEvent.PAY)
              .and().withExternal()
                .source(OrderState.PAID).target(OrderState.SHIPPED).event(OrderEvent.SHIP)
                .guard(ctx -> ctx.getMessageHeader("inventory") != null)
              .and().withExternal()
                .source(OrderState.SHIPPED).target(OrderState.DELIVERED).event(OrderEvent.DELIVER);
    }
}
```

Pros: enterprise-grade. Hierarchical and parallel states. Persistence
adapters (JPA, Redis, Kafka). Distributed coordination via Zookeeper.
Cons: significant boilerplate. Tied to Spring.

References: [Baeldung, *A Guide to the Spring State Machine Project*](https://www.baeldung.com/spring-state-machine), [Spring StateMachine reference docs](https://docs.spring.io/spring-statemachine/docs/current/reference/).

## Approach 4: squirrel-foundation

A lightweight HSM library for Java with fluent builders, method-call
actions, and optional declarative annotations. A good middle ground between
the enum form and Spring StateMachine.

```java
// Sketch; squirrel-foundation imports and dependency setup omitted.
@StateMachineParameters(stateType = OrderState.class,
                         eventType = OrderEvent.class,
                         contextType = Order.class)
public class OrderStateMachine extends AbstractUntypedStateMachine {

    public void onPlace(OrderState from, OrderState to, OrderEvent event, Order order) {
        order.timestamp = Instant.now();
    }

    public void onPaidEntry(OrderState from, OrderState to, OrderEvent event, Order order) {
        order.notifyShipping();
    }
}

UntypedStateMachineBuilder builder = StateMachineBuilderFactory
    .create(OrderStateMachine.class);
builder.externalTransition().from(OrderState.DRAFT).to(OrderState.PLACED)
    .on(OrderEvent.PLACE).callMethod("onPlace");
builder.externalTransition().from(OrderState.PLACED).to(OrderState.PAID).on(OrderEvent.PAY);
builder.onEntry(OrderState.PAID).callMethod("onPaidEntry");

UntypedStateMachine sm = builder.newStateMachine(OrderState.DRAFT);
sm.fire(OrderEvent.PLACE, order);
```

Pros: supports hierarchy, has a fluent builder, entry/exit hooks, and async
processing. Smaller surface than Spring StateMachine.
Cons: less ecosystem integration.

Reference: [squirrel-foundation README](https://github.com/hekailiang/squirrel),
[user guide](http://hekailiang.github.io/squirrel/).

## Picking between the approaches

| You need | Use |
|---|---|
| Small, in-memory only | Approach 1 (enum) |
| Per-state behavior beyond transitions | Approach 2 (GoF) |
| Hierarchy, parallel regions, persistence, Spring shop | Approach 3 (Spring SM) |
| Lightweight HSM with hooks, no Spring | Approach 4 (squirrel) |

Avoid the temptation to "just add Spring StateMachine" for a 5-state
in-memory machine. The enum form scales further than people expect.

## Sources

- [Baeldung: Implementing Simple State Machines with Java Enums](https://www.baeldung.com/java-enum-simple-state-machine)
- [Baeldung: A Guide to the Spring State Machine Project](https://www.baeldung.com/spring-state-machine)
- [Spring StateMachine reference docs](https://docs.spring.io/spring-statemachine/docs/current/reference/)
- [Spring StateMachine project page](https://spring.io/projects/spring-statemachine/)
- [squirrel-foundation (GitHub)](https://github.com/hekailiang/squirrel)
- [squirrel-foundation user guide](http://hekailiang.github.io/squirrel/)
- [Refactoring Guru: State pattern (Java)](https://refactoring.guru/design-patterns/state/java/example)
