export function App() {
  const [count, setCount] = useState(0);
  return (
    <View style={{ flexDirection: "column", padding: 2 }}>
      <Text style={{ color: "#FF5733" }}>Count: {count}</Text>
      <Button onPress={() => setCount((c) => c + 1)}>Increment</Button>
    </View>
  );
}
