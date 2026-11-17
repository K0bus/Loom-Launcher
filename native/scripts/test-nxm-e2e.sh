#!/usr/bin/env bash
# Test bout en bout des liens nxm:// sur une session de bureau réelle.
#
# Reproduit ce que fait un navigateur (Brave/Chrome) quand on clique « Mod Manager Download » :
# il demande à xdg-desktop-portal d'ouvrir l'URI, qui lance le gestionnaire par défaut.
# Le script vérifie que :
#   1. l'association x-scheme-handler/nxm pointe vers loom-native.desktop ;
#   2. le portail ne garde pas en mémoire une autre application (desktop-used-apps) ;
#   3. un lien factice, envoyé par le portail, atteint bien Loom natif (journal native-nxm.log) ;
#   4. une seconde instance est transmise à la première (instance unique).
#
# Usage : native/scripts/test-nxm-e2e.sh   (une session graphique + D-Bus sont nécessaires)
set -u
LOG="${XDG_DATA_HOME:-$HOME/.local/share}/loom/logs/native-nxm.log"
BIN="${LOOM_NATIVE_BIN:-$(dirname "$0")/../target/release/loom-native}"
FAKE="nxm://skyrimspecialedition/mods/1/files/1?key=e2e-test&expires=9999999999&user_id=1"
ok=0; ko=0
check() { if [ "$2" = "0" ]; then echo "  ✓ $1"; ok=$((ok+1)); else echo "  ✗ $1"; ko=$((ko+1)); fi; }

echo "== Association"
handler=$(gio mime x-scheme-handler/nxm 2>/dev/null | sed -n '1p')
echo "$handler" | grep -q "loom-native.desktop"; check "gestionnaire par défaut = loom-native.desktop" $?
grep -q "^x-scheme-handler/nxm=.*loom-launcher" "$HOME/.config/mimeapps.list" 2>/dev/null; [ $? -ne 0 ]; check "plus de référence loom-launcher (Tauri) dans mimeapps.list" $?

echo "== Mémoire du portail"
mem=$(gdbus call --session --dest org.freedesktop.impl.portal.PermissionStore --object-path /org/freedesktop/impl/portal/PermissionStore \
  --method org.freedesktop.impl.portal.PermissionStore.Lookup desktop-used-apps x-scheme-handler/nxm 2>/dev/null)
echo "$mem" | grep -q "loom-launcher"; [ $? -ne 0 ]; check "le portail ne privilégie pas loom-launcher" $?

echo "== Lien envoyé par le portail"
before=$(wc -l < "$LOG" 2>/dev/null || echo 0)
started=0
if ! gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.ListNames >/dev/null 2>&1; then echo "  ! pas de bus de session"; exit 2; fi
if ! ss -xl 2>/dev/null | grep -q "loom-native.sock"; then "$BIN" >/dev/null 2>&1 & started=$!; sleep 8; fi
gdbus call --session --dest org.freedesktop.portal.Desktop --object-path /org/freedesktop/portal/desktop \
  --method org.freedesktop.portal.OpenURI.OpenURI "" "$FAKE" "{}" >/dev/null 2>&1
sleep 4
new=$(tail -n +"$((before+1))" "$LOG" 2>/dev/null)
echo "$new" | grep -q "lien reçu"; check "Loom natif a reçu le lien (journal)" $?
echo "$new" | grep -q "key=\*\*\*"; check "la clé de téléchargement est masquée dans le journal" $?
ps -eo args | grep -E "[l]oom-launcher nxm" >/dev/null; [ $? -ne 0 ]; check "l'app Tauri n'a pas été lancée" $?

echo "== Instance unique"
n=$(ps -eo args | grep -cE "[l]oom-native($| nxm)")
[ "$n" -le 2 ]; check "pas de seconde instance ouverte (processus loom-native : $n)" $?

[ "$started" != "0" ] && kill "$started" 2>/dev/null
echo; echo "$ok réussis, $ko échoués"; [ "$ko" -eq 0 ]
