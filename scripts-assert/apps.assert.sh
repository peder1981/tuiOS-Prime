#!/usr/bin/env bash
# Ciclo completo do tuios-apps em tmpdir (sem QEMU): R1-R13 em env isolado.
set -euo pipefail

RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$RAIZ/apps/tuios-apps/tuios-apps"
command -v python3 >/dev/null || { echo "FALHA: python3 ausente"; exit 1; }
[ -f "$BIN" ] || { echo "FALHA: entrypoint ausente: $BIN"; exit 1; }

W="$(mktemp -d /tmp/tuios-apps-test.XXXXXX)"
trap 'rm -rf "$W"' EXIT

export TUIOS_APPS_SYSTEM="$W/sistema"
export TUIOS_APPS_USER="$W/usuario"
mkdir -p "$TUIOS_APPS_SYSTEM" "$TUIOS_APPS_USER"

falha() { echo "FALHA: $*"; exit 1; }

# 1. app de exemplo
APP="$W/origem/ola-tuios"
mkdir -p "$APP"
cat > "$APP/tuios-app.toml" << 'EOF'
nome = "ola-tuios"
versao = "1.0.0"
descricao = "Primeiro app tuiOS"
entry = "ola.prw"
categoria = "exemplo"
EOF
cat > "$APP/ola.prw" << 'EOF'
User Function Ola()
    ConOut("OLA-TUIOS-OK")
Return .T.
EOF

# 2. validar (R1/R13)
"$BIN" validar "$APP" >/dev/null || falha "validar"

# 3. empacotar (R9)
( cd "$W/origem" && "$BIN" empacotar ola-tuios -o "$W/ola-tuios-1.0.0.tar.gz" >/dev/null ) \
  || falha "empacotar"
[ -f "$W/ola-tuios-1.0.0.tar.gz" ] || falha "tarball nao gerado"

# 4. adicionar (R10) — destino padrao = usuario
"$BIN" adicionar "$W/ola-tuios-1.0.0.tar.gz" >/dev/null || falha "adicionar"
[ -f "$TUIOS_APPS_USER/ola-tuios/tuios-app.toml" ] || falha "app nao instalado no dir usuario"

# 5. listar --json parseavel (R6)
JSON="$("$BIN" listar --json)" || falha "listar --json"
echo "$JSON" | python3 -c '
import json, sys
dados = json.load(sys.stdin)
assert any(a["nome"] == "ola-tuios" and a["origem"] == "usuario" for a in dados), dados
' || falha "JSON sem ola-tuios"

# 6. rodar: aceita 0 (executou) ou 4 (advplc ausente fora da ISO) (R7/R8)
set +e
"$BIN" rodar ola-tuios >/dev/null 2>&1
RC=$?
set -e
case "$RC" in
  0|4) ;;
  *) falha "rodar retornou $RC (esperado 0 ou 4)" ;;
esac

# 7. exit codes (R2/R3)
set +e
"$BIN" info fantasma >/dev/null 2>&1; RC1=$?
"$BIN" validar "$W/nao-existe" >/dev/null 2>&1; RC2=$?
set -e
[ "$RC1" -eq 3 ] || falha "info inexistente exit=$RC1 (esperado 3)"
[ "$RC2" -eq 2 ] || falha "validar invalido exit=$RC2 (esperado 2)"

# 8. remover (R11)
"$BIN" remover ola-tuios >/dev/null || falha "remover"
[ ! -e "$TUIOS_APPS_USER/ola-tuios" ] || falha "remover nao removeu o app"

# 9. sombra: app do usuario esconde o do sistema (R5)
mkdir -p "$TUIOS_APPS_SYSTEM/ola-tuios"
cp "$APP/tuios-app.toml" "$APP/ola.prw" "$TUIOS_APPS_SYSTEM/ola-tuios/"
mkdir -p "$TUIOS_APPS_USER/ola-tuios"
sed 's/1.0.0/9.9.9/' "$APP/tuios-app.toml" > "$TUIOS_APPS_USER/ola-tuios/tuios-app.toml"
cp "$APP/ola.prw" "$TUIOS_APPS_USER/ola-tuios/"
VERSAO="$("$BIN" listar --json | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d[0]["versao"], d[0]["origem"])')"
[ "$VERSAO" = "9.9.9 usuario" ] || falha "sombra usuario>sistema nao vale (obteve: $VERSAO)"

# 10. menu com dialog fake (R14/R20) — sem terminal, fila de respostas
FAKE="$W/fake-dialog"
cat > "$FAKE" << 'FDEOF'
#!/usr/bin/env python3
import os, sys, pathlib
log = pathlib.Path(os.environ["FAKE_LOG"])
with log.open("a", encoding="utf-8") as f:
    f.write(" ".join(sys.argv[1:]).replace("\n", " ") + "\n")
fila = pathlib.Path(os.environ["FAKE_QUEUE"])
linhas = fila.read_text(encoding="utf-8").splitlines() if fila.exists() else []
if not linhas:
    sys.exit(1)
rc, _, saida = linhas[0].partition("|")
fila.write_text("\n".join(linhas[1:]), encoding="utf-8")
sys.stdout.write(saida)
sys.exit(int(rc))
FDEOF
chmod +x "$FAKE"
: > "$W/dialog.log"
printf '0|listar\n0|voltar\n0|sair\n' > "$W/dialog.queue"
TUIOS_DIALOG="$FAKE" FAKE_LOG="$W/dialog.log" FAKE_QUEUE="$W/dialog.queue" \
  "$BIN" menu >/dev/null || falha "menu"
grep -q -- "--menu" "$W/dialog.log" || falha "menu nao invocou dialog"
grep -q "ola-tuios" "$W/dialog.log" || falha "menu nao listou ola-tuios"

# 11. origem remota sem rede: file:// + indice (R21, R23-R26)
"$BIN" remover ola-tuios >/dev/null || falha "limpeza p/ file://"
"$BIN" empacotar "$APP" -o "$W/pacote.tar.gz" >/dev/null || falha "empacotar p/ file://"
"$BIN" adicionar "file://$W/pacote.tar.gz" >/dev/null || falha "adicionar file://"
"$BIN" listar --json | grep -q '"nome": "ola-tuios"' || falha "file:// nao instalou"
"$BIN" remover ola-tuios >/dev/null || falha "remover apos file://"
SHA="$(sha256sum "$W/pacote.tar.gz" | cut -d' ' -f1)"
cat > "$W/indice-remoto.toml" << INDEOF
[[app]]
nome = "ola-tuios"
versao = "1.0.0"
descricao = "Primeiro app tuiOS"
url = "file://$W/pacote.tar.gz"
sha256 = "$SHA"
INDEOF
"$BIN" atualizar-indice "file://$W/indice-remoto.toml" >/dev/null || falha "atualizar-indice"
"$BIN" buscar primeiro | grep -q "ola-tuios" || falha "buscar no indice"
"$BIN" instalar ola-tuios >/dev/null || falha "instalar do indice"
"$BIN" listar --json | grep -q '"origem": "usuario"' || falha "instalar do indice nao instalou"

echo "APPS-ASSERT-OK"
