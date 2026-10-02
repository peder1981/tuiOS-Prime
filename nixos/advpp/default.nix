{ pkgs, ... }:

pkgs.stdenv.mkDerivation {
  pname = "advplc";
  version = "1.0.0";
  
  src = /home/peder/Projetos/AdvPP;
  
  nativeBuildInputs = [ pkgs.go pkgs.gopls ];
  buildInputs = [ pkgs.go ];
  
  GO111MODULE = "on";
  CGO_ENABLED = "0";
  
  buildPhase = ''
    go build -ldflags="-X main.version=${version}" -o advplc ./cmd/advplc
  '';
  
  installPhase = ''
    mkdir -p $out/bin
    mv advplc $out/bin/
    
    # Criar manual
    mkdir -p $out/share/man/man1
    cat > $out/share/man/man1/advplc.1 << 'MAN'
.TH ADVPLC 1 "AdvPP Compiler" "Version 1.0.0"
.SH NAME
advplc \- Compilador AdvPL/TLPP
.SH SYNOPSIS
\fBadvplc\fR \fICOMMAND\fR \fIFILE\fR [\fIOPTIONS\fR]
.SH COMMANDS
.TP
\fBrun\fR \fIFILE\fR
Compila e executa um arquivo AdvPL/TLPP
.TP
\fBcompile\fR \fIFILE\fR [\fB-o\fR \fIOUTPUT\fR]
Compila para bytecode
.TP
\fBexec\fR \fIBYTESOCDE\fR
Executa bytecode compilado
.TP
\fBcheck\fR \fIFILE\fR
Valida sintaxe sem executar
.TP
\fBserve\fR \fIFILE\fR [\fB--port\fR \fIPORT\fR]
Modo web
.TP
\fBbuild\fR \fIFILE\fR [\fB-o\fR \fIOUTPUT\fR] [\fB--gui\fR]
Build standalone
.TP
\fBdebug\fR \fIFILE\fR
Debug DAP
.TP
\fBast\fR \fIFILE\fR
Mostra AST
.TP
\fBbytecode\fR \fIFILE\fR
Mostra bytecode
.SH OPTIONS
.TP
\fB--include\fR, \fB-I\fR \fIPATH\fR
Adiciona path de includes
.TP
\fB--define\fR, \fB-D\fR \fINAME=VALUE\fR
Define simbolo do preprocessor
.TP
\fB--ui\fR
Habilita UI Fyne
.TP
\fB--headless\fR
Desabilita UI (padrao)
.TP
\fB-o\fR \fIFILE\fR
Arquivo de saida
.TP
\fB--port\fR \fIN\fR
Porta para modo web
.TP
\fB-w\fR, \fB--watch\fR
Hot reload no modo web
.TP
\fB--gui\fR
Marca como app desktop (Fyne window)
.SH EXAMPLES
.TP
\fBadvplc run hello.prw\fR
Compila e executa hello.prw
.TP
\fBadvplc compile hello.prw -o hello.bytecode\fR
Compila para bytecode
.TP
\fBadvplc exec hello.bytecode\fR
Executa bytecode
.TP
\fBadvplc check program.prw --include ./includes\fR
Valida sintaxe
.TP
\fBadvplc serve app.prw --port 9000 --watch\fR
Modo web com hot reload
.TP
\fBadvplc build app.prw -o app --gui\fR
Build standalone desktop
.SH AUTHOR
Peder Munksgaard
.SH "REPORTING BUGS"
Report bugs to https://github.com/peder1981/AdvPP/issues
MAN
  '';
  
  meta = with pkgs.lib; {
    description = "AdvPP - Compilador AdvPL/TLPP em Go";
    homepage = "https://github.com/peder1981/AdvPP";
    license = licenses.mit;
    platforms = platforms.linux;
    mainProgram = "advplc";
  };
}
