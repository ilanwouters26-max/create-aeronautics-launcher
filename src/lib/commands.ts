// Arbre des commandes vanilla pour l'autocomplétion de la console.
// Un nœud est soit un littéral (name / choices), soit un argument typé (arg).

import { BIOMES, DIFFICULTIES, EFFECTS, ENCHANTMENTS, ENTITIES, GAMEMODES, GAMERULES, ITEMS, SELECTORS, STRUCTURES, WEATHERS } from "./mc-data";

export type ArgType =
  | "player"
  | "entity"
  | "item"
  | "block"
  | "effect"
  | "enchantment"
  | "gamerule"
  | "structure"
  | "biome"
  | "int"
  | "float"
  | "coord"
  | "text"
  | "bool"
  | "name";

export interface Node {
  /** Littéral unique. */
  name?: string;
  /** Plusieurs littéraux acceptés au même endroit. */
  choices?: string[];
  /** Argument typé (valeurs dynamiques ou libres). */
  arg?: ArgType;
  /** Nom affiché pour un argument (<joueur>, <quantité>...). */
  label?: string;
  desc?: string;
  next?: Node[];
  /** Un argument "text" avale le reste de la ligne. */
  rest?: boolean;
}

const player = (next?: Node[], label = "joueur"): Node => ({ arg: "player", label, next });
const int = (label: string, next?: Node[]): Node => ({ arg: "int", label, next });
const float = (label: string, next?: Node[]): Node => ({ arg: "float", label, next });
const text = (label = "message"): Node => ({ arg: "text", label, rest: true });
const coords = (next?: Node[]): Node => ({ arg: "coord", label: "x", next: [{ arg: "coord", label: "y", next: [{ arg: "coord", label: "z", next }] }] });
const lit = (name: string, next?: Node[], desc?: string): Node => ({ name, next, desc });
const choice = (choices: string[], next?: Node[]): Node => ({ choices, next });

const scoreboardPlayerOps = (op: string) => lit(op, [player([{ arg: "name", label: "objectif", next: [int("valeur")] }])]);

export const COMMANDS: Node[] = [
  lit("gamemode", [choice(GAMEMODES, [player()])], "Changer le mode de jeu"),
  lit("defaultgamemode", [choice(GAMEMODES)], "Mode de jeu par défaut"),
  lit("gamerule", [{ arg: "gamerule", label: "règle", next: [{ arg: "bool", label: "valeur" }] }], "Lire ou changer une règle"),
  lit("give", [player([{ arg: "item", label: "objet", next: [int("quantité")] }])], "Donner un objet"),
  lit("clear", [player([{ arg: "item", label: "objet", next: [int("max")] }])], "Vider l'inventaire"),
  lit("tp", [{ arg: "player", label: "cible", next: [{ arg: "player", label: "destination" }, coords()] }], "Téléporter"),
  lit("teleport", [{ arg: "player", label: "cible", next: [{ arg: "player", label: "destination" }, coords()] }], "Téléporter"),
  lit("kill", [{ arg: "entity", label: "cible" }], "Tuer une entité"),
  lit("time", [lit("set", [choice(["day", "night", "noon", "midnight"]), int("ticks")]), lit("add", [int("ticks")]), lit("query", [choice(["daytime", "gametime", "day"])])], "Heure du monde"),
  lit("weather", [choice(WEATHERS, [int("durée")])], "Météo"),
  lit("difficulty", [choice(DIFFICULTIES)], "Difficulté"),
  lit("op", [player()], "Donner les droits admin"),
  lit("deop", [player()], "Retirer les droits admin"),
  lit("kick", [player([text("raison")])], "Expulser un joueur"),
  lit("ban", [player([text("raison")])], "Bannir un joueur"),
  lit("ban-ip", [{ arg: "player", label: "joueur ou IP", next: [text("raison")] }], "Bannir une IP"),
  lit("pardon", [player()], "Débannir"),
  lit("pardon-ip", [{ arg: "name", label: "ip" }], "Débannir une IP"),
  lit("banlist", [choice(["players", "ips"])], "Liste des bannis"),
  lit("whitelist", [lit("on"), lit("off"), lit("list"), lit("reload"), lit("add", [player()]), lit("remove", [player()])], "Liste blanche"),
  lit("say", [text()], "Message à tous"),
  lit("tell", [player([text()])], "Message privé"),
  lit("msg", [player([text()])], "Message privé"),
  lit("w", [player([text()])], "Message privé"),
  lit("me", [text()], "Action"),
  lit("tellraw", [player([text("json")])], "Message JSON"),
  lit("title", [player([lit("title", [text("texte")]), lit("subtitle", [text("texte")]), lit("actionbar", [text("texte")]), lit("clear"), lit("reset"), lit("times", [int("fadeIn", [int("stay", [int("fadeOut")])])])])], "Titre à l'écran"),
  lit("effect", [lit("give", [player([{ arg: "effect", label: "effet", next: [int("secondes", [int("niveau", [{ arg: "bool", label: "particules cachées" }])])] }])]), lit("clear", [player([{ arg: "effect", label: "effet" }])])], "Effets de potion"),
  lit("enchant", [player([{ arg: "enchantment", label: "enchantement", next: [int("niveau")] }])], "Enchanter l'objet tenu"),
  lit("xp", [choice(["add", "set"], [player([int("quantité", [choice(["levels", "points"])])])]), lit("query", [player([choice(["levels", "points"])])])], "Expérience"),
  lit("experience", [choice(["add", "set"], [player([int("quantité", [choice(["levels", "points"])])])]), lit("query", [player([choice(["levels", "points"])])])], "Expérience"),
  lit("setworldspawn", [coords()], "Spawn du monde"),
  lit("spawnpoint", [player([coords()])], "Point de réapparition"),
  lit("summon", [{ arg: "entity", label: "entité", next: [coords()] }], "Invoquer une entité"),
  lit("seed", undefined, "Graine du monde"),
  lit("list", [lit("uuids")], "Joueurs connectés"),
  lit("save-all", [lit("flush")], "Sauvegarder le monde"),
  lit("save-on", undefined, "Activer la sauvegarde auto"),
  lit("save-off", undefined, "Désactiver la sauvegarde auto"),
  lit("stop", undefined, "Arrêter le serveur"),
  lit("reload", undefined, "Recharger datapacks et config"),
  lit("help", [{ arg: "name", label: "commande" }], "Aide"),
  lit("locate", [lit("structure", [{ arg: "structure", label: "structure" }]), lit("biome", [{ arg: "biome", label: "biome" }]), lit("poi", [{ arg: "name", label: "poi" }])], "Localiser"),
  lit("worldborder", [lit("set", [float("taille", [int("secondes")])]), lit("add", [float("delta", [int("secondes")])]), lit("center", [{ arg: "coord", label: "x", next: [{ arg: "coord", label: "z" }] }]), lit("get"), lit("damage", [choice(["amount", "buffer"], [float("valeur")])]), lit("warning", [choice(["distance", "time"], [int("valeur")])])], "Bordure du monde"),
  lit("setblock", [coords([{ arg: "block", label: "bloc", next: [choice(["destroy", "keep", "replace"])] }])], "Placer un bloc"),
  lit("fill", [coords([coords([{ arg: "block", label: "bloc", next: [choice(["destroy", "hollow", "keep", "outline", "replace"])] }])])], "Remplir une zone"),
  lit("execute", [lit("as", [{ arg: "entity", label: "entité", next: [lit("run", [text("commande")])] }]), lit("at", [{ arg: "entity", label: "entité", next: [lit("run", [text("commande")])] }]), lit("run", [text("commande")])], "Exécuter en contexte"),
  lit("scoreboard", [
    lit("objectives", [lit("list"), lit("add", [{ arg: "name", label: "objectif", next: [{ arg: "name", label: "critère" }] }]), lit("remove", [{ arg: "name", label: "objectif" }]), lit("setdisplay", [choice(["sidebar", "list", "belowName"], [{ arg: "name", label: "objectif" }])])]),
    lit("players", [lit("list", [player()]), scoreboardPlayerOps("set"), scoreboardPlayerOps("add"), scoreboardPlayerOps("remove"), lit("reset", [player([{ arg: "name", label: "objectif" }])])]),
  ], "Scores"),
  lit("team", [lit("list"), lit("add", [{ arg: "name", label: "équipe" }]), lit("remove", [{ arg: "name", label: "équipe" }]), lit("join", [{ arg: "name", label: "équipe", next: [player()] }]), lit("leave", [player()]), lit("empty", [{ arg: "name", label: "équipe" }])], "Équipes"),
  lit("spectate", [{ arg: "entity", label: "cible", next: [player()] }], "Mode spectateur sur une cible"),
  lit("damage", [{ arg: "entity", label: "cible", next: [float("dégâts")] }], "Infliger des dégâts"),
  lit("forceload", [lit("add", [{ arg: "coord", label: "x", next: [{ arg: "coord", label: "z" }] }]), lit("remove", [choice(["all"]), { arg: "coord", label: "x", next: [{ arg: "coord", label: "z" }] }]), lit("query")], "Chunks forcés"),
  lit("datapack", [lit("list"), lit("enable", [{ arg: "name", label: "datapack" }]), lit("disable", [{ arg: "name", label: "datapack" }])], "Datapacks"),
  lit("function", [{ arg: "name", label: "fonction" }], "Lancer une fonction"),
  lit("tick", [lit("query"), lit("rate", [float("tps")]), lit("freeze"), lit("unfreeze"), lit("step", [int("ticks")]), lit("sprint", [int("ticks")])], "Vitesse des ticks"),
  lit("setidletimeout", [int("minutes")], "Déconnexion des inactifs"),
  lit("stopsound", [player()], "Couper les sons"),
  lit("perf", [lit("start"), lit("stop")], "Profilage"),
  lit("debug", [lit("start"), lit("stop")], "Débogage"),
];

export interface Suggestion {
  value: string;
  desc?: string;
}

export interface Completion {
  suggestions: Suggestion[];
  /** Position du début du mot en cours dans la chaîne. */
  wordStart: number;
  /** Aide contextuelle : argument attendu. */
  hint: string;
}

interface Context {
  players: string[];
}

function argValues(arg: ArgType, ctx: Context): string[] {
  switch (arg) {
    case "player":
      return [...ctx.players, ...SELECTORS];
    case "entity":
      return [...ctx.players, ...SELECTORS, ...ENTITIES];
    case "item":
      return ITEMS;
    case "block":
      return ITEMS;
    case "effect":
      return EFFECTS;
    case "enchantment":
      return ENCHANTMENTS;
    case "gamerule":
      return GAMERULES;
    case "structure":
      return STRUCTURES;
    case "biome":
      return BIOMES;
    case "bool":
      return ["true", "false"];
    case "coord":
      return ["~", "~ ~ ~"];
    default:
      return [];
  }
}

function matchesLiteral(node: Node, token: string): boolean {
  if (node.name) return node.name === token;
  if (node.choices) return node.choices.includes(token);
  return false;
}

function nodeAccepts(node: Node, token: string): boolean {
  if (node.name || node.choices) return matchesLiteral(node, token);
  return token.length > 0;
}

function nodeLabel(node: Node): string {
  if (node.name) return node.name;
  if (node.choices) return node.choices.join("|");
  return `<${node.label ?? node.arg}>`;
}

/** Suggestions pour la saisie courante (sans le "/" initial). */
export function complete(raw: string, ctx: Context): Completion {
  const input = raw.startsWith("/") ? raw.slice(1) : raw;
  const offset = raw.length - input.length;
  const tokens = input.split(" ");
  const current = tokens[tokens.length - 1];
  const wordStart = offset + input.length - current.length;
  let candidates: Node[] = COMMANDS;
  for (let i = 0; i < tokens.length - 1; i++) {
    const tok = tokens[i];
    const literal = candidates.find((n) => (n.name || n.choices) && matchesLiteral(n, tok));
    const node = literal ?? candidates.find((n) => n.arg && nodeAccepts(n, tok));
    if (!node) return { suggestions: [], wordStart, hint: "" };
    if (node.rest) return { suggestions: [], wordStart, hint: `<${node.label ?? "texte"}>` };
    candidates = node.next ?? [];
  }
  const lower = current.toLowerCase();
  const stripped = lower.startsWith("minecraft:") ? lower.slice("minecraft:".length) : lower;
  const seen = new Set<string>();
  const suggestions: Suggestion[] = [];
  const push = (value: string, desc?: string) => {
    if (seen.has(value)) return;
    seen.add(value);
    suggestions.push({ value, desc });
  };
  for (const n of candidates) {
    if (n.name) {
      if (n.name.toLowerCase().startsWith(lower)) push(n.name, n.desc);
    } else if (n.choices) {
      for (const c of n.choices) if (c.toLowerCase().startsWith(lower)) push(c);
    } else if (n.arg) {
      for (const v of argValues(n.arg, ctx)) {
        const vl = v.toLowerCase();
        if (vl.startsWith(lower) || vl.startsWith(stripped)) push(v);
      }
    }
  }
  const prefixed = suggestions.filter((s) => s.value.toLowerCase().startsWith(lower));
  const ordered = prefixed.length ? prefixed : suggestions;
  const hint = candidates.map(nodeLabel).slice(0, 6).join("  ");
  return { suggestions: ordered.slice(0, 40), wordStart, hint };
}

/** Vrai si, après ce mot, d'autres arguments sont possibles (on ajoute un espace). */
export function hasContinuation(raw: string): boolean {
  const input = raw.startsWith("/") ? raw.slice(1) : raw;
  const tokens = input.trim().split(" ");
  let candidates: Node[] = COMMANDS;
  for (const tok of tokens) {
    const literal = candidates.find((n) => (n.name || n.choices) && matchesLiteral(n, tok));
    const node = literal ?? candidates.find((n) => n.arg && nodeAccepts(n, tok));
    if (!node) return false;
    candidates = node.next ?? [];
  }
  return candidates.length > 0;
}
