# Decisões e revisão técnica

## Âmbito do MVP

Companion local, de uso experimental, que controla um dispositivo Spotify autorizado. O valor a explorar é um comando desktop pequeno, nativo e com poucas distrações. Não é um motor de streaming nem uma substituição integral do Spotify.

## Fronteiras dos módulos

```text
Slint / thread da interface
     │ comandos (fila limitada a 4)
     ▼
worker / thread de rede e estado
     ├── auth: PKCE + listener loopback temporário
     ├── api: HTTPS oficial + renovação + controlo de limites
     └── model: dados de reprodução e permissões
     │ snapshots de estado
     ▼
event loop Slint
```

O cliente HTTP é reutilizado. O pedido HTTP tem timeout total de 12 s e de ligação de 5 s. O trabalho de rede não bloqueia a interface. O cliente HTTP pode criar threads internas; não se afirma que a aplicação tenha apenas duas threads.

## Autenticação

- Aplicação pública desktop, sem Client Secret.
- Verificador PKCE e `state` com 32 bytes de aleatoriedade cada; desafio SHA-256 em Base64 URL sem padding.
- Listener exclusivo em `127.0.0.1:8888`, aberto antes de iniciar o navegador.
- Método GET, caminho de retorno, parâmetro `state` único, tamanho do pedido e prazo do login verificados.
- Pedidos inválidos recebem HTTP 400 e não consomem o fluxo pendente.
- Cancelamento por sinal atómico; a troca de tokens já em curso está limitada pelo timeout de rede.
- Os tokens de acesso ficam em memória. Os tokens de renovação ficam no Gestor de Credenciais do Windows, separados por finalidade e associados ao Client ID. Os corpos de erro remotos ficam fora dos logs.
- A renovação conserva o token anterior quando o Spotify não devolve um novo. Um token de renovação recusado como `invalid_grant` é removido.
- **Desligar** termina a reprodução controlada pela app. **Esquecer autorização** remove as credenciais OAuth guardadas neste PC. A revogação externa é efetuada na conta Spotify.

## Pedidos, memória e interface

- Consulta de reprodução a cada 20 s quando ativa e 60 s quando em pausa.
- A barra de progresso avança localmente a 1 Hz apenas durante reprodução, sem pedido de rede por segundo.
- A lista de dispositivos é consultada no login e por Atualizar; não em todas as consultas periódicas.
- Depois de um comando, a interface espera um segundo e confirma o estado na API. A API pode ter consistência eventual; um segundo não é uma garantia do Spotify.
- Erros de rede provocam espera crescente até 300 s. HTTP 429 respeita `Retry-After`; quota esgotada suspende o polling. HTTP 403 exige atualização explícita ou correção das permissões.
- Os comandos de mudança de faixa não são repetidos automaticamente. Dispositivos restritos e ações `disallows` são verificados.
- Respostas limitadas a 1 MiB; respostas de tokens limitadas a 64 KiB; até 32 dispositivos em memória.
- A biblioteca carrega até quatro capas por página, reduzidas a 200 píxeis e mantidas em memória. O áudio continua a cargo do Soloist ou do dispositivo Spotify Connect.
- Renderer Slint por software e funcionalidades de acessibilidade ativadas. Só as funcionalidades selecionadas são compiladas para o executável, apesar de o lockfile poder incluir dependências condicionais para outras plataformas.

## Problemas detetados e corrigidos na revisão

1. Dimensões fixas entravam em conflito com os mínimos de janela no Slint. Passaram a dimensões preferidas.
2. O progresso ficava congelado entre consultas à API. Foi acrescentado um temporizador local que para em pausa.
3. Os controlos eram reativados imediatamente após os comandos. Agora aguardam uma confirmação posterior.
4. A atualização do estado repunha a seleção da lista de dispositivos. O modelo visual só é substituído quando a lista muda.
5. O polling e a atualização manual partilhavam uma distinção baseada no relógio. Passaram a comandos separados, evitando pedidos desnecessários da lista de dispositivos.
6. O tema dos controlos podia seguir o Windows e contrastar com a janela escura. O tema escuro é definido explicitamente.

## Validação ainda dependente da conta

- Criar aplicação Spotify e concluir OAuth real com a conta Premium autorizada.
- Confirmar as permissões e a disponibilidade dos endpoints para essa aplicação.
- Experimentar renovação real de token e recusa/revogação de consentimento.
- Reproduzir, pausar, mudar de faixa e transferir entre dispositivos reais.
- Medir uma sessão ligada e comparar consumo total quando o Spotify desktop também está aberto.
- Avaliar funcionamento sem rede, recuperação após suspensão do computador e alterações realizadas noutro dispositivo.

## Antes de eventual publicação

Escolher outro nome público, rever marca e atribuição do Spotify, avaliar se o caso de uso cumpre a política de valor independente e se precisa de autorização, rever licenças das dependências e preparar instalador e assinatura. Estes passos não são apresentados como concluídos. A atribuição Slint já está em Ajuda > Sobre, ao abrigo da alternativa Royalty-free 2.0.
