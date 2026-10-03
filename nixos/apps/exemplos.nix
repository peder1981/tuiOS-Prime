# App-exemplo de fabrica do Nivel C — copiado para /opt/tuios/apps na live (R16).
# tmpfiles C so copia se o destino nao existe/esta vazio (man tmpfiles.d).
{ runCommand }:

runCommand "tuios-exemplos" { } ''
  mkdir -p $out
  cp -r ${../../apps/exemplos}/. $out/
''
