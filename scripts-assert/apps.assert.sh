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

echo "APPS-ASSERT-OK"
