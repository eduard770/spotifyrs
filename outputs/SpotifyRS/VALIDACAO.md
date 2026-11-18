## Arranque do Soloist na interface

A versão Windows encontrou o executável local e ligou-se, através de **Iniciar e ligar Soloist**, ao processo oficial já em execução. Esse processo continuou ativo depois de fechar a janela de teste.

Um processo simulado confirmou que o SpotifyRS consegue iniciar o motor sem consola, detetar um dispositivo ainda por emparelhar, reutilizar uma instância e terminar o processo que iniciou. Passaram os 18 testes automatizados e a análise Clippy. O arranque do motor oficial com uma chave real guardada fica por validar.

## Redesenho da janela no Windows

A compilação usa a [correção do Slint para zonas invalidadas pelo Windows](https://github.com/slint-ui/slint/pull/13568), aplicada ao backend Winit da versão 1.18.1. Verifiquei «Ligações» e «Explorar» após maximizar, voltar ao tamanho normal e restaurar depois de minimizar. A versão corrigida manteve os controlos visíveis nesses ensaios. A imagem do problema original não foi reproduzida de forma consistente.

Os 18 testes automatizados passaram com o backend corrigido.

## Interface de pesquisa e seleção de faixas

Passaram 18 testes automatizados. Os testes incluem a pesquisa com resposta simulada, a validação de links Spotify e o envio do URI selecionado por WebSocket. Na janela em demonstração foram verificados o filtro por artista, a escolha de uma faixa nos resultados e na fila, e a mudança para a faixa seguinte.

O áudio real com uma conta Spotify e o consumo conjunto com o Soloist ainda estão por medir. Os dados abaixo são da versão anterior da interface. A configuração atual está em [SOLOIST.md](SOLOIST.md).

# Validação do SpotifyRS 0.1

Data: 7 de novembro de 2026. Protótipo Windows x86-64, Rust 1.99.1 (GNU), Slint 1.18.1. Compilação release otimizada para tamanho, LTO thin, renderer por software, acessibilidade ativada.

## Resultado

- Compilação release concluída e executável aberto no Windows.
- 13 testes automatizados aprovados, sem falhas.
- Análise Clippy dos alvos e testes concluída sem avisos; formatação Rust aplicada.
- Demonstração verificada na janela real: reprodução, pausa, anterior, seguinte, progresso local, saída e regresso ao ecrã de ligação.
- Menu Ajuda > Sobre e atribuição Slint verificados visualmente.
- Corrigidos problemas visuais do preenchimento da barra e do espaço do rodapé; capturas finais na pasta screenshots.
- Executável: 10.86 MiB. A inspeção das importações identificou apenas DLLs do Windows, sem runtime GNU externo exigido pelo executável testado.

## Medições do executável final

Cada ensaio contém 60 amostras a intervalos de aproximadamente um segundo, após três segundos de aquecimento. Windows reportado: Microsoft Windows NT 10.0.26300.0. Computador com 16 processadores lógicos. Janela com dimensão preferida de 920 × 720 píxeis lógicos, além da moldura e menu do Windows.

| Métrica | Ecrã inicial em repouso | Demonstração com progresso ativo |
| --- | ---: | ---: |
| Memória residente média (MiB) | 25.637 | 27.657 |
| Memória residente máxima nas amostras (MiB) | 25.656 | 27.672 |
| Pico residente do processo (MiB) | 25.656 | 27.703 |
| Memória privada média (MiB) | 6.673 | 7.266 |
| CPU média normalizada (%) | 0.0032 | 0.0371 |
| Máximo de threads observado | 13 | 13 |

O ensaio de demonstração foi feito após experimentar o diálogo Sobre. O ensaio em repouso usou uma nova instância. As medições são apenas do processo SpotifyRS; não incluem o Spotify, o navegador ou outros processos do computador. CPU normalizada significa percentagem da capacidade conjunta dos processadores lógicos. MiB = 1 048 576 bytes. Memória privada e memória residente são métricas diferentes, não parcelas a somar.

Estes ensaios breves **não demonstram o consumo numa sessão autenticada**, estabilidade durante horas, ausência de fugas de memória, desempenho noutro hardware ou menor consumo do que todos os frameworks alternativos. Não são benchmarks de áudio nem análise de conteúdo Spotify. A meta exploratória de menos de 150 MiB foi cumprida apenas nos cenários locais medidos.

Os dados brutos CSV e resumos JSON estão em measurements. Todos os resultados desta tabela correspondem a este SHA-256:

E20673E5B8F228C99897A6DB5BF6DF097137589F578B7C121D23F71477BB0276

## Cobertura dos testes automatizados

| Área | Casos cobertos |
| --- | --- |
| PKCE | Vetor de teste RFC 7636 e parâmetros de autorização sem Client Secret |
| Retorno OAuth | State incorreto/duplicado, método e caminho inválidos, código e recusa |
| Renovação | Preservação do token quando não há rotação e deteção de uma autorização revogada |
| Preferências e credenciais | Gravação e reabertura, validação do endereço local e leitura e remoção de uma credencial de teste no Windows |
| Modelos | Item nulo, campos opcionais, ações e dispositivos restritos, formatação de duração |
| Web API com servidor local simulado | HTTP 204, 401, 403, 404, 429, Retry-After e quota esgotada |
| Worker | Demonstração, reprodução, faixa seguinte e limpeza do estado ao sair |

## Por validar com a conta

O utilizador ainda não tem aplicação Spotify criada. Não foi feito login real, nem autorizada uma conta, nem controlado o Spotify Premium aberto no computador. Faltam o Client ID e o consentimento no navegador.

Depois da configuração descrita no README: validar login, listagem de dispositivos, comandos, transferência, renovação de sessão e recuperação de rede. Só então medir RAM/CPU de uma sessão ligada. Se o áudio sair do Spotify desktop no mesmo computador, medir também o consumo conjunto dos dois processos.

