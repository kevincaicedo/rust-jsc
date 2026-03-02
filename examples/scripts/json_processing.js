// Generate a large JSON dataset
function generateData(count) {
    const data = [];
    for (let i = 0; i < count; i++) {
        data.push({
            id: i,
            name: "user_" + i,
            email: "user" + i + "@example.com",
            age: 20 + (i % 50),
            active: i % 3 !== 0,
            scores: [i * 10, i * 20, i * 30],
            address: {
                city: "City_" + (i % 10),
                zip: String(10000 + i),
                country: i % 2 === 0 ? "US" : "UK"
            }
        });
    }
    return data;
}

// Transform pipeline
function processData(data) {
    return data
        .filter(user => user.active)
        .map(user => ({
            ...user,
            fullName: user.name.toUpperCase(),
            avgScore: user.scores.reduce((a, b) => a + b, 0) / user.scores.length,
            region: user.address.country === "US" ? "North America" : "Europe"
        }))
        .sort((a, b) => b.avgScore - a.avgScore)
        .slice(0, 100);
}

// Serialize and parse back
function roundTrip(data) {
    const json = JSON.stringify(data);
    const parsed = JSON.parse(json);
    return { byteLength: json.length, itemCount: parsed.length };
}

const raw = generateData(1000);
const processed = processData(raw);
const result = roundTrip(processed);

export default result;
