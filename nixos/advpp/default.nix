# Pacote Nix para o compilador AdvPP (advplc 4.4.0).
#
# Estratégia: instala o binário pré-compilado e corrige o interpretador
# ELF com autoPatchelfHook (o binário foi ligado contra /lib64 do host,
# que não existe no NixOS). Evita compilar o Go no sandbox (sem rede
# para módulos).
#
# Binário versionado no repo (nixos/advpp/advplc) → build 100% puro,
# viabilizando CI. Fallback: checkout local AdvPP (exige --impure).
# Para outra origem: pkgs.callPackage ./nixos/advpp { advppBin = /caminho; };
{ lib, stdenv, autoPatchelfHook, glibc
, advppBin ? (if builtins.pathExists ./advplc then ./advplc
              else /home/peder/Projetos/AdvPP/advplc) }:

stdenv.mkDerivation {
  pname = "advplc";
  version = "4.4.0";

  src = advppBin;
  dontUnpack = true;

  nativeBuildInputs = [ autoPatchelfHook ];
  buildInputs = [ glibc ];

  installPhase = ''
    mkdir -p $out/bin
    install -m755 $src $out/bin/advplc

    mkdir -p $out/share/man/man1
    cat > $out/share/man/man1/advplc.1 << 'MAN'
.TH ADVPLC 1 "Compilador AdvPP" "Versão 4.4.0"
.SH NOME
advplc \- Compilador AdvPL/TLPP
.SH SINOPSE
\fBadvplc\fR \fICOMANDO\fR \fIARQUIVO\fR [\fIOPÇÕES\fR]
.SH COMANDOS
.TP
\fBrun\fR \fIARQUIVO\fR
Compila e executa um arquivo AdvPL/TLPP
.TP
\fBcompile\fR \fIARQUIVO\fR [\fB-o\fR \fISAÍDA\fR]
Compila o fonte para bytecode
.TP
\fBexec\fR \fIBYTECODE\fR
Executa um arquivo de bytecode compilado
.TP
\fBcheck\fR \fIARQUIVO\fR
Valida a sintaxe sem executar
.TP
\fBserve\fR \fIARQUIVO\fR [\fB--port\fR \fIPORTA\fR]
Executa o programa em modo web
.TP
\fBbuild\fR \fIARQUIVO\fR [\fB-o\fR \fISAÍDA\fR] [\fB--gui\fR]
Compila para executável standalone
.TP
\fBdebug\fR \fIARQUIVO\fR
Atende sessão DAP (depuração) via stdio
.TP
\fBast\fR \fIARQUIVO\fR
Exibe a árvore sintática (AST)
.TP
\fBbytecode\fR \fIARQUIVO\fR
Exibe o bytecode compilado
.SH OPÇÕES
.TP
\fB--include\fR, \fB-I\fR \fICAMINHO\fR
Adiciona caminho de includes (pode repetir)
.TP
\fB--define\fR, \fB-D\fR \fINOME=VALOR\fR
Define símbolo do pré-processador
.TP
\fB--ui\fR
Habilita interface gráfica (Fyne)
.TP
\fB--headless\fR
Desabilita interface gráfica (padrão)
.TP
\fB-o\fR \fIARQUIVO\fR
Arquivo de saída do comando compile
.TP
\fB--port\fR \fIN\fR
Porta do modo web (padrão 8080)
.TP
\fB-w\fR, \fB--watch\fR
No modo web: recompila ao salvar (hot reload)
.TP
\fB--gui\fR
No build: marca o programa como aplicativo desktop
.SH EXEMPLOS
.TP
\fBadvplc run ola.prw\fR
Compila e executa ola.prw
.TP
\fBadvplc compile ola.prw -o ola.bytecode\fR
Compila para bytecode
.TP
\fBadvplc exec ola.bytecode\fR
Executa o bytecode
.TP
\fBadvplc check prog.prw --include ./includes\fR
Valida a sintaxe
.TP
\fBadvplc serve app.prw --port 9000 --watch\fR
Modo web com recarga automática
.TP
\fBadvplc build app.prw -o app --gui\fR
Gera executável desktop standalone
.SH AUTOR
Peder Munksgaard
.SH "RELATANDO PROBLEMAS"
Relate problemas em https://github.com/peder1981/AdvPP/issues
MAN
  '';

  meta = with lib; {
    description = "AdvPP - Compilador AdvPL/TLPP em Go";
    homepage = "https://github.com/peder1981/AdvPP";
    license = licenses.mit;
    platforms = platforms.linux;
    mainProgram = "advplc";
  };
}
