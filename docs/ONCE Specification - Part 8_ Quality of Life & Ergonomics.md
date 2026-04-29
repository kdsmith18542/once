# **ONCE Specification \- Part 8: Quality of Life & Ergonomics**

| Document ID | ONCE-008 |
| :---- | :---- |
| **Title** | Quality of Life & Ergonomics |
| **Version** | 1.0 |
| **Date** | 2025-10-20 |
| **Status** | Draft |
| **Related Docs** | ONCE-001, ONCE-002, ONCE-003, ONCE-006 |

## **1\. Introduction**

This document specifies a suite of features for the Once language and its tooling that are explicitly designed to enhance developer productivity, reduce cognitive load, and make the language easy to learn and delightful to use. These features address common, high-friction "pain points" in modern software development.

While the core language guarantees correctness (memory safety, race freedom), these ergonomic features ensure that writing correct code is also the path of least resistance.

## **2\. Data Handling & Transformation**

A significant portion of development involves transforming and validating data. Once provides features to make this process declarative, safe, and free of boilerplate.

### **2.1. Schema-Driven Data Hydration**

* **Pain Point:** Manually parsing and validating untyped data (e.g., JSON, YAML) is repetitive, verbose, and a common source of runtime errors.  
* **Solution:** A schema declaration provides a declarative mapping from an untyped data source to a typed Once struct. The compiler uses this schema to generate high-performance, safe parsing and validation code.

**Specification:**

// Target Once struct  
type User \= { id: Int, username: Str, is\_active: Bool }

// Declarative mapping from a json::Value  
schema JsonToUser from json::Value for User {  
  id maps from ".user\_id", required, as Int;  
  username maps from ".user\_name", required, as Str;  
  is\_active maps from ".status.active", default: true, as Bool;  
}

// Usage  
fn create\_user(data: json::Value) \-\> Result\<User, schema::Error\> {  
  // Compiler generates the entire validation and mapping logic.  
  let user \= hydrate\<JsonToUser\>(data)?;  
  Ok(user)  
}

The hydrate function returns a structured schema::Error on failure, pinpointing the exact field and reason for the validation error.

### **2.2. Pipeline Operator**

* **Pain Point:** Deeply nested function calls for data transformation (g(f(h(x)))) are hard to read and write.  
* **Solution:** The pipeline operator |\> allows for a more natural, left-to-right composition of functions.

**Specification:**

expr ::= expr "|\>" expr

The expression x |\> f(y) is equivalent to f(x, y). x |\> f is equivalent to f(x).

**Example:**

let result \= data  
  |\> parse\_json()?  
  |\> validate\_records()  
  |\> filter\_active\_users()  
  |\> count();

## **3\. Error Handling & Debugging**

Once aims to make errors transparent and debugging an intuitive process.

### **3.1. Integrated Error Context**

* **Pain Point:** Errors propagated from deep call stacks often lose valuable context, making debugging difficult.  
* **Solution:** A try block automatically captures contextual data and attaches it to any error that propagates through it.

**Specification:**

fn load\_user\_config(path: Str) \-\> Result\<Config, Error\> \!io {  
  try {  
    using file \= File::open(path)?; // Error here captures \`path\`  
    let text \= file.read\_to\_string()?;  
    let config \= parse\_config(text)?;  
    Ok(config)  
  }  
}

If File::open fails, the resulting Error object will contain a structured trace including context: { path: "/path/to/file.toml" }, which can be programmatically inspected or printed.

## **4\. Testing & Verification**

Testing code with side effects is a major challenge. Once integrates a solution directly into the language.

### **4.1. Test-Time Effect Overrides**

* **Pain Point:** Unit testing functions with I/O or network effects requires complex mocking frameworks or dependency injection, which pollutes application code.  
* **Solution:** The test runner allows the implementation of an effect to be declaratively overridden within the scope of a test.

**Specification:**

// Production code  
fn get\_user\_name(id: Int) \-\> Result\<Str, Error\> \!\[net\] {  
  http::get("\[https://api.example.com/users/\](https://api.example.com/users/){id}")?.body\_string()  
}

\#\[test\]  
fn test\_get\_user\_name\_success() {  
  // Override the \`net\` effect for this test only.  
  override std::effects::net with mock\_net {  
    on http::get("\[https://api.example.com/users/1\](https://api.example.com/users/1)") respond with {  
      status: 200,  
      body: "{\\"name\\": \\"Alice\\"}"  
    }  
  }

  // This call now hits the mock, not the real network.  
  let name \= get\_user\_name(1).unwrap();  
  assert\_eq(name, "{\\"name\\": \\"Alice\\"}");  
}

This allows the *exact same production code* to be tested without modification, ensuring tests are fast, deterministic, and can run offline.

### **4.2. Doctests**

* **Pain Point:** Documentation and examples often become outdated as code evolves.  
* **Solution:** Code examples inside documentation comments are compiled and run as part of the standard test suite.

**Specification:**

/// Calculates the sum of a vector of integers.  
///  
/// \#\# Example  
///  
/// \`\`\`  
/// let numbers \= Vec::from(\[1, 2, 3\]);  
/// let total \= sum(numbers);  
/// assert\_eq(total, 6);  
/// \`\`\`  
fn sum(xs: Vec\<Int\>) \-\> Int {  
  // ... implementation ...  
}

Running once test will execute the code within the \`\`\` block, failing the build if the assertion fails.

## **5\. Learning & Code Comprehension**

Once is designed to be easy to learn, with tooling that actively helps the developer understand the code and the compiler's reasoning.

### **5.1. Type Hole \_**

* **Pain Point:** When learning a new language or API, developers often know *what* they want to do but not the exact type required in a given position.  
* **Solution:** A developer can use \_ as an expression. The compiler will treat this as an error but will report the *inferred type* it expected at that position, along with the context.

**Example:**

let numbers \= Vec::from(\[1, 2, 3\]);  
let total \= numbers.fold(0, \_);

**Compiler Output:**

Error: Found type hole \_ at main.onc:2:31.  
Expected type: fn(Int, Int) \-\> Int  
The fold function expects an initial value and a reducer function.

### **5.2. Explainer & Doctor Tooling**

* **Pain Point:** Complex compiler features like borrow checkers or effect systems can feel like a "black box."  
* **Solution:** The once CLI includes a suite of commands to make the compiler's reasoning transparent and to automatically fix common issues.  
  * **once explain \--regions \<file:line\>**: Renders a visual graph of inferred memory regions.  
  * **once explain \--effects \<file:line\>**: Shows the call graph and reasoning for an inferred effect signature.  
  * **once explain \--linearity \<file:line\>**: Traces the "ownership chain" of a linear value to show where it was created, moved, and consumed.  
  * **once fix \--imports**: Automatically adds, removes, and sorts necessary import statements.  
  * **once fix \--consumes**: Inserts a missing using block or consume() call for an unconsumed linear value.

These features make the compiler a collaborative partner rather than a gatekeeper, accelerating the learning process and making development more productive.