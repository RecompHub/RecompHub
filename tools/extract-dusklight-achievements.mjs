// Extrai as conquistas do Dusklight (TwilitRealm/dusklight, CC0) do código-fonte
// e grava catalog/trophies/dusklight.json.
//
//   node tools/extract-dusklight-achievements.mjs [tag]     (padrão: v2.0.3)
//
// As definições ficam em src/dusk/achievements.cpp, em makeEntries():
//   { { "key", "Nome", "Descrição", AchievementCategory::X, isCounter, goal, 0, false }, check, {} }
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const tag = process.argv[2] ?? "v2.0.3";
const url = `https://raw.githubusercontent.com/TwilitRealm/dusklight/${tag}/src/dusk/achievements.cpp`;
const source = await (await fetch(url)).text();

// Literal C++ ("..." "..." concatenados), com escapes simples.
const str = String.raw`((?:"(?:[^"\\]|\\.)*"\s*)+)`;
const entry = new RegExp(
  String.raw`\{\s*${str},\s*${str},\s*${str},\s*AchievementCategory::(\w+),\s*(true|false),\s*([^,]+),\s*[^,]+,\s*(?:true|false)\s*\}`,
  "g",
);
const unquote = (s) =>
  [...s.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((m) => JSON.parse(`"${m[1]}"`)).join("");

const trophies = [];
for (const m of source.matchAll(entry)) {
  const isCounter = m[5] === "true";
  const goal = Number(m[6].trim());
  if (isCounter && !Number.isFinite(goal)) {
    throw new Error(`meta não numérica em ${unquote(m[1])}: ${m[6]}`);
  }
  trophies.push({
    key: unquote(m[1]),
    title: unquote(m[2]),
    description: unquote(m[3]),
    category: m[4].toLowerCase(),
    goal: isCounter ? goal : 0,
  });
}
if (trophies.length === 0) throw new Error("nenhuma conquista encontrada; o formato mudou?");

const out = {
  source: `TwilitRealm/dusklight ${tag} (CC0-1.0), src/dusk/achievements.cpp`,
  trophies,
};
const path = fileURLToPath(new URL("../catalog/trophies/dusklight.json", import.meta.url));
writeFileSync(path, JSON.stringify(out, null, 2) + "\n");
console.log(`${trophies.length} conquistas → ${path}`);
