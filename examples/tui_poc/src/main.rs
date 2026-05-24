//! TUI Proof of Concept - Hand-written reference implementation.
//!
//! This file serves as a reference for what the generated code looks like.
//! The actual compiled binary uses src/generated_main.rs which is generated
//! from app.tsx by the build.rs script.
//!
//! To regenerate the output after modifying app.tsx, run:
//!   cargo build
//!
//! Source (app.tsx):
//!   export function App() {
//!     const [count, setCount] = useState(0);
//!     return (
//!       <View style={{ flexDirection: "column", padding: 2 }}>
//!         <Text style={{ color: "#FF5733" }}>Count: {count}</Text>
//!         <Button onPress={() => setCount((c) => c + 2)}>Increment</Button>
//!       </View>
//!     );
//!   }
