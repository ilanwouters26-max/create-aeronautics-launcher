// Listes vanilla pour l'autocomplétion (identifiants sans le préfixe minecraft:).

export const GAMEMODES = ["survival", "creative", "adventure", "spectator"];
export const DIFFICULTIES = ["peaceful", "easy", "normal", "hard"];
export const WEATHERS = ["clear", "rain", "thunder"];
export const SELECTORS = ["@a", "@p", "@r", "@s", "@e"];

export const GAMERULES = [
  "announceAdvancements", "commandBlockOutput", "disableElytraMovementCheck", "disableRaids", "doDaylightCycle", "doEntityDrops",
  "doFireTick", "doImmediateRespawn", "doInsomnia", "doLimitedCrafting", "doMobLoot", "doMobSpawning", "doPatrolSpawning",
  "doTileDrops", "doTraderSpawning", "doVinesSpread", "doWardenSpawning", "doWeatherCycle", "drowningDamage",
  "enderPearlsVanishOnDeath", "fallDamage", "fireDamage", "forgiveDeadPlayers", "freezeDamage", "globalSoundEvents",
  "keepInventory", "lavaSourceConversion", "logAdminCommands", "maxCommandChainLength", "maxCommandForkCount",
  "maxEntityCramming", "mobExplosionDropDecay", "mobGriefing", "naturalRegeneration", "playersNetherPortalCreativeDelay",
  "playersNetherPortalDefaultDelay", "playersSleepingPercentage", "projectilesCanBreakBlocks", "randomTickSpeed",
  "reducedDebugInfo", "sendCommandFeedback", "showDeathMessages", "snowAccumulationHeight", "spawnChunkRadius", "spawnRadius",
  "spectatorsGenerateChunks", "tntExplosionDropDecay", "universalAnger", "waterSourceConversion",
];

export const EFFECTS = [
  "speed", "slowness", "haste", "mining_fatigue", "strength", "instant_health", "instant_damage", "jump_boost", "nausea",
  "regeneration", "resistance", "fire_resistance", "water_breathing", "invisibility", "blindness", "night_vision", "hunger",
  "weakness", "poison", "wither", "health_boost", "absorption", "saturation", "glowing", "levitation", "luck", "unluck",
  "slow_falling", "conduit_power", "dolphins_grace", "bad_omen", "hero_of_the_village", "darkness", "trial_omen", "raid_omen",
  "wind_charged", "weaving", "oozing", "infested",
];

export const ENCHANTMENTS = [
  "aqua_affinity", "bane_of_arthropods", "binding_curse", "blast_protection", "breach", "channeling", "density", "depth_strider",
  "efficiency", "feather_falling", "fire_aspect", "fire_protection", "flame", "fortune", "frost_walker", "impaling", "infinity",
  "knockback", "looting", "loyalty", "luck_of_the_sea", "lure", "mending", "multishot", "piercing", "power", "projectile_protection",
  "protection", "punch", "quick_charge", "respiration", "riptide", "sharpness", "silk_touch", "smite", "soul_speed", "sweeping_edge",
  "swift_sneak", "thorns", "unbreaking", "vanishing_curse", "wind_burst",
];

export const ENTITIES = [
  "allay", "armadillo", "armor_stand", "arrow", "axolotl", "bat", "bee", "blaze", "boat", "bogged", "breeze", "camel", "cat",
  "cave_spider", "chicken", "cod", "cow", "creeper", "dolphin", "donkey", "drowned", "elder_guardian", "ender_dragon", "enderman",
  "endermite", "evoker", "experience_orb", "fox", "frog", "ghast", "glow_squid", "goat", "guardian", "hoglin", "horse", "husk",
  "illusioner", "iron_golem", "item", "lightning_bolt", "llama", "magma_cube", "minecart", "mooshroom", "mule", "ocelot", "panda",
  "parrot", "phantom", "pig", "piglin", "piglin_brute", "pillager", "polar_bear", "pufferfish", "rabbit", "ravager", "salmon",
  "sheep", "shulker", "silverfish", "skeleton", "skeleton_horse", "slime", "sniffer", "snow_golem", "spider", "squid", "stray",
  "strider", "tadpole", "tnt", "trader_llama", "tropical_fish", "turtle", "vex", "villager", "vindicator", "wandering_trader",
  "warden", "witch", "wither", "wither_skeleton", "wolf", "zoglin", "zombie", "zombie_horse", "zombie_villager", "zombified_piglin",
];

export const STRUCTURES = [
  "ancient_city", "bastion_remnant", "buried_treasure", "desert_pyramid", "end_city", "fortress", "igloo", "jungle_pyramid",
  "mansion", "mineshaft", "monument", "nether_fossil", "ocean_ruin_cold", "ocean_ruin_warm", "pillager_outpost", "ruined_portal",
  "shipwreck", "stronghold", "swamp_hut", "trail_ruins", "trial_chambers", "village_desert", "village_plains", "village_savanna",
  "village_snowy", "village_taiga",
];

export const BIOMES = [
  "badlands", "bamboo_jungle", "basalt_deltas", "beach", "birch_forest", "cherry_grove", "cold_ocean", "crimson_forest",
  "dark_forest", "deep_dark", "deep_ocean", "desert", "dripstone_caves", "end_highlands", "eroded_badlands", "flower_forest",
  "forest", "frozen_ocean", "frozen_peaks", "frozen_river", "grove", "ice_spikes", "jagged_peaks", "jungle", "lukewarm_ocean",
  "lush_caves", "mangrove_swamp", "meadow", "mushroom_fields", "nether_wastes", "ocean", "old_growth_birch_forest",
  "old_growth_pine_taiga", "old_growth_spruce_taiga", "plains", "river", "savanna", "savanna_plateau", "snowy_beach",
  "snowy_plains", "snowy_slopes", "snowy_taiga", "soul_sand_valley", "sparse_jungle", "stony_peaks", "stony_shore",
  "sunflower_plains", "swamp", "taiga", "the_end", "the_void", "warm_ocean", "warped_forest", "windswept_forest",
  "windswept_gravelly_hills", "windswept_hills", "windswept_savanna", "wooded_badlands",
];

export const ITEMS = [
  "diamond", "diamond_block", "diamond_sword", "diamond_pickaxe", "diamond_axe", "diamond_shovel", "diamond_hoe",
  "diamond_helmet", "diamond_chestplate", "diamond_leggings", "diamond_boots", "netherite_ingot", "netherite_scrap",
  "netherite_sword", "netherite_pickaxe", "netherite_axe", "netherite_shovel", "netherite_hoe", "netherite_helmet",
  "netherite_chestplate", "netherite_leggings", "netherite_boots", "netherite_block", "netherite_upgrade_smithing_template",
  "iron_ingot", "iron_block", "iron_nugget", "iron_sword", "iron_pickaxe", "iron_axe", "iron_shovel", "iron_hoe", "iron_helmet",
  "iron_chestplate", "iron_leggings", "iron_boots", "gold_ingot", "gold_block", "gold_nugget", "golden_apple",
  "enchanted_golden_apple", "golden_carrot", "golden_sword", "golden_pickaxe", "golden_helmet", "golden_chestplate",
  "golden_leggings", "golden_boots", "copper_ingot", "copper_block", "raw_iron", "raw_gold", "raw_copper", "coal", "charcoal",
  "coal_block", "emerald", "emerald_block", "lapis_lazuli", "lapis_block", "redstone", "redstone_block", "redstone_torch",
  "quartz", "quartz_block", "amethyst_shard", "ancient_debris", "stick", "string", "feather", "flint", "leather", "bone",
  "bone_meal", "gunpowder", "blaze_rod", "blaze_powder", "ender_pearl", "ender_eye", "ghast_tear", "nether_star", "nether_wart",
  "slime_ball", "magma_cream", "phantom_membrane", "prismarine_shard", "prismarine_crystals", "nautilus_shell", "heart_of_the_sea",
  "shulker_shell", "dragon_breath", "dragon_egg", "totem_of_undying", "elytra", "trident", "mace", "bow", "crossbow", "arrow",
  "spectral_arrow", "tipped_arrow", "shield", "fishing_rod", "shears", "flint_and_steel", "compass", "recovery_compass", "clock",
  "spyglass", "map", "filled_map", "book", "writable_book", "written_book", "enchanted_book", "experience_bottle", "name_tag",
  "lead", "saddle", "bucket", "water_bucket", "lava_bucket", "milk_bucket", "powder_snow_bucket", "axolotl_bucket", "bread",
  "apple", "cooked_beef", "cooked_porkchop", "cooked_chicken", "cooked_mutton", "cooked_cod", "cooked_salmon", "baked_potato",
  "carrot", "potato", "beetroot", "melon_slice", "pumpkin_pie", "cookie", "cake", "sweet_berries", "glow_berries", "honey_bottle",
  "mushroom_stew", "rabbit_stew", "suspicious_stew", "chorus_fruit", "dried_kelp", "rotten_flesh", "spider_eye", "potion",
  "splash_potion", "lingering_potion", "glass_bottle", "torch", "soul_torch", "lantern", "soul_lantern", "campfire", "furnace",
  "blast_furnace", "smoker", "crafting_table", "anvil", "enchanting_table", "bookshelf", "chest", "ender_chest", "barrel",
  "shulker_box", "hopper", "dropper", "dispenser", "observer", "piston", "sticky_piston", "lever", "comparator", "repeater",
  "tnt", "bed", "white_bed", "red_bed", "oak_log", "oak_planks", "spruce_log", "birch_log", "dark_oak_log", "oak_sapling",
  "stone", "cobblestone", "deepslate", "cobbled_deepslate", "granite", "diorite", "andesite", "dirt", "grass_block", "sand",
  "gravel", "glass", "glass_pane", "obsidian", "crying_obsidian", "netherrack", "glowstone", "sea_lantern", "end_stone",
  "end_rod", "beacon", "conduit", "respawn_anchor", "lodestone", "bell", "ladder", "scaffolding", "rail", "powered_rail",
  "minecart", "chest_minecart", "oak_boat", "wheat", "wheat_seeds", "sugar", "sugar_cane", "egg", "paper", "wool",
  "white_wool", "ink_sac", "glow_ink_sac", "clay_ball", "brick", "snowball", "fire_charge", "firework_rocket", "firework_star",
  "zombie_spawn_egg", "creeper_spawn_egg", "villager_spawn_egg", "command_block", "structure_block", "barrier", "light",
  "debug_stick", "knowledge_book", "bundle", "wind_charge", "breeze_rod", "trial_key", "ominous_trial_key", "ominous_bottle",
];
