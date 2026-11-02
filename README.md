# 🎮 Loom Launcher — Universal Desktop Gaming Platform

Loom est un lanceur de jeux moderne et modulaire ciblant en priorité absolue **Linux** (avec prise en charge native de **Wine, Proton-GE, GameMode, MangoHud, Gamescope**) et prêt pour un déploiement multiplateforme (Windows, macOS).

Construit avec **Tauri v2**, **Vue 3 (Composition API / `<script setup>`)**, **TypeScript**, **Tailwind CSS**, et **Pinia**.

---

## 🌟 Caractéristiques Clés

### 🎨 Direction Artistique & Thème Gaming Sombre
- **Fonds :** Nuances profondes `zinc-950` et `zinc-900/60` avec bordures fines subtiles `border-zinc-800/80` et glassmorphism (`backdrop-blur-md`).
- **Accents :** Teintes dorées et ambres (`text-amber-400`, `bg-amber-500`, halos `shadow-amber-500/20`).
- **Typographie :** *Inter* pour l'interface fluide et *JetBrains Mono* pour les métriques techniques & logs.

### 🧭 Shell Bureau Natif (Desktop App Layout)
- **Top Bar Flottante intégrée (Titlebar Tauri v2) :**
  - Contrôles de fenêtre (traffic lights macOS / Linux : Fermer, Réduire, Agrandir avec appels natifs `@tauri-apps/api/window` et zone de drag `data-tauri-drag-region`).
  - Navigation dans l'historique (Précédent / Suivant).
  - Pilules de bascule de section : **Bibliothèque**, **Fiche Jeu**, **Studio Minecraft** (avec icône cube), **Comptes**, **Paramètres**.
  - Recherche instantanée fuzzy centrale avec raccourci clavier bureau `<kbd>⌘K</kbd>` / `<kbd>Ctrl+K</kbd>`.
  - Accès direct au gestionnaire de téléchargements, notifications et profil utilisateur compact.
- **Système de Toast Notification Bureau :** Notifier flottant en bas à droite pour le retour d'état instantané (démarrage/arrêt de Steam, lancement du jeu, synchronisation cloud, sauvegarde des runners).

### 🏠 Dashboard (Accueil)
1. **Hero Cinématique (21:9) :** Affichage grand format du dernier jeu joué avec fond assombri en dégradé, temps de jeu total, succès débloqués et gros bouton d'action doré `JOUER MAINTENANT`.
2. **Carrousel Horizontal :** Reprise rapide des derniers jeux joués sous forme d'affiches 3:4 avec bouton de lancement rapide au survol.
3. **Widget Stats & Activité :** Temps de jeu cumulé sur la semaine et jauges d'espace disque disponible sur les partitions NVMe/SSD de jeux.
4. **Timeline des actualités :** Flux des derniers patch notes et annonces officielles des jeux installés.

### 📚 Bibliothèque Unifiée (LibraryView)
1. **Header & Recherche :** Barre de recherche instantanée multi-critères (titre, genre).
2. **Filtres Avancés (Modale) :** Filtrage par plateforme (Steam, Epic, GOG, Minecraft), runner (Natif, Proton-GE, Proton Experimental, Wine Staging), genre et critères de tri.
3. **Bandeau de rappels récents :** Accès direct aux jeux les plus fréquents.
4. **Grille de vignettes (3:4) :** Format affiche de film avec overlay cinématique, bouton de lancement rapide et configuration rapide du runner.

### 🎬 Fiche Jeu Détaillée (Cinématique)
1. **Backdrop 21:9 haute résolution** en dégradé vers le fond `zinc-950` avec jaquette, badges, métriques de jeu et boutons d'action.
2. **Bouton conditionnel intelligent :** Détecte automatiquement Minecraft et affiche un accès direct vers le **Minecraft Studio**.
3. **Onglet Aperçu & Succès :** Résumé du jeu, crédits, captures d'écran 16:9 et barre de progression des succès débloqués.
4. **Onglet Configuration & Runner :** Sélection à chaud du moteur de compatibilité Linux (Proton GE, Proton Experimental, Wine Staging), commutateurs pour *Feral GameMode*, *MangoHud*, *Gamescope*, chemin du `WINEPREFIX` et arguments personnalisés.

### ⛏️ Minecraft Studio
1. **Sous-navigation :** *Mes Instances* vs *Catalogue Modpacks*.
2. **Catalogue Modpacks :** Intégration unifiée des répertoires **Modrinth** et **CurseForge** avec recherche, filtres et installation en 1 clic.
3. **Assistant de Création d'Instance (Wizard) :** Modale guidée en 4 étapes :
   - Étape 1 : Nom & icône personnalisée
   - Étape 2 : Version de Minecraft (Release 1.21.4, 1.20.1, Snapshot...)
   - Étape 3 : Choix du Mod Loader (Fabric, NeoForge, Forge, Quilt, Vanilla)
   - Étape 4 : Allocation mémoire RAM (Go) et JVM
4. **Détail d'Instance :**
   - **Mods :** Liste filtrable, commutateurs ON/OFF instantanés, détection des mises à jour disponibles et ouverture du dossier.
   - **Mondes & Shaders :** Visualisation des sauvegardes, packs de shaders (Oculus/Iris) et resource packs.
   - **Console & Logs :** Terminal en temps réel avec coloration par niveau de criticité (`INFO`, `WARN`, `ERROR`), copie presse-papier et vidage.

### 🔑 Hub de Comptes & Services
- Association des comptes de jeu : **Steam**, **Epic Games**, **GOG**, et **Microsoft Xbox Live**.
- **OAuth2 Device Code Flow :** Modal d'appairage interactive générant le code utilisateur officiel Microsoft avec lien direct vers `microsoft.com/link` et confirmation automatique.

---

## 🏗️ Architecture du Projet

```
Loom/
├── src/
│   ├── assets/styles/main.css      # Design system, glassmorphism, scrollbars
│   ├── components/
│   │   ├── games/
│   │   │   ├── GameCard.vue        # Affiche 3:4 avec survol dynamique
│   │   │   ├── GameFilterModal.vue # Modale filtres avancés (Plateforme, Runner, Genre)
│   │   │   └── GameRunnerModal.vue # Modale réglage Runner / MangoHud / GameMode
│   │   ├── layout/
│   │   │   ├── AppHeader.vue       # Header global avec recherche et status bar
│   │   │   └── AppSidebar.vue      # Sidebar fixe avec navigation et sondes système
│   │   ├── minecraft/
│   │   │   ├── InstanceGeneralTab.vue # Mondes, shaders, textures
│   │   │   ├── InstanceLogsTab.vue    # Console JVM en direct
│   │   │   ├── InstanceModsTab.vue    # Gestionnaire de mods avec toggles
│   │   │   ├── InstanceWizardModal.vue # Wizard étape par étape façon CurseForge
│   │   │   └── ModpackCard.vue        # Carte modpack Modrinth/CurseForge 1-clic
│   │   └── ui/                     # Composants atomiques réutilisables
│   │       ├── UiBadge.vue
│   │       ├── UiButton.vue
│   │       ├── UiInput.vue
│   │       ├── UiModal.vue
│   │       ├── UiProgressBar.vue
│   │       ├── UiSwitch.vue
│   │       └── UiTabs.vue
│   ├── models/                     # Interfaces TypeScript
│   │   ├── account.ts
│   │   ├── game.ts
│   │   ├── minecraft.ts
│   │   └── system.ts
│   ├── router/index.ts             # Routes Vue Router
│   ├── services/
│   │   └── ipc.ts                  # Abstraction IPC Tauri v2 avec fallback Mock
│   ├── stores/                     # Stores Pinia avec données ultra-réalistes
│   │   ├── accountsStore.ts
│   │   ├── gamesStore.ts
│   │   ├── minecraftStore.ts
│   │   └── systemStore.ts
│   ├── views/
│   │   ├── AccountsView.vue
│   │   ├── DashboardView.vue
│   │   ├── GameDetailView.vue
│   │   ├── LibraryView.vue
│   │   ├── MinecraftStudioView.vue
│   │   └── SettingsView.vue
│   ├── App.vue
│   └── main.ts
├── src-tauri/
│   ├── Cargo.toml                  # Dépendances Rust (tauri v2, serde)
│   ├── tauri.conf.json             # Configuration de la fenêtre et bundle desktop
│   └── src/
│       ├── lib.rs                  # Handlers Tauri IPC (launch_game, system_stats)
│       └── main.rs
├── package.json
├── tailwind.config.js
└── vite.config.ts
```

---

## 🚀 Démarrage & Exécution

### 1. Mode Développement Web (avec IPC émulé)
```bash
npm install
npm run dev
```
Accédez à l'application via `http://localhost:5173/`. Le pont IPC bascule automatiquement sur les mocks ultra-réalistes et les données sont interactives.

### 2. Compilation et Vérification TypeScript
```bash
npm run build
```

### 3. Exécution avec Tauri v2 (Bureau Desktop Linux)
Lorsque Cargo/Rust est installé sur la machine :
```bash
npm run tauri dev
```
