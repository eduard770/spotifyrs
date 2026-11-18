# Reprodução local com Soloist

No Windows, o SpotifyRS pode iniciar e controlar o Soloist a partir da página **Ligações**. A fila local e a reprodução por link usam a ligação ao Soloist. A pesquisa por nome usa uma autorização separada do catálogo Spotify.

1. Gera a tua chave Soloist no [painel oficial](https://developer.spotify.com/dashboard/soloist). Não a coloques no chat.
2. Descarrega e extrai o executável Windows oficial seguindo os [downloads Soloist](https://developer.spotify.com/documentation/soloist/reference/downloads-and-updates).
3. Abre `spotifyrs.exe` e vai a **Ligações**. Confirma o caminho do `soloist.exe`, introduz a chave no campo **Chave Soloist** e escolhe **Iniciar e ligar Soloist**.
4. Se for a primeira vez, abre o Spotify no telemóvel na mesma rede e escolhe **SpotifyRS local** nos dispositivos disponíveis. O SpotifyRS deteta a ligação assim que o Soloist fica emparelhado.
5. Para reutilizar a chave, marca **Guardar chave neste PC** ao iniciar o Soloist pela app. Nas próximas vezes, podes deixar o campo vazio. **Esquecer chave** remove a entrada do Gestor de Credenciais do Windows.

O SpotifyRS guarda o caminho do executável e o endereço local ao fechar. Marca **Iniciar Soloist ao abrir** para o SpotifyRS o ligar automaticamente nas próximas aberturas. A chave só é guardada quando selecionas **Guardar chave neste PC** e carregas em **Iniciar e ligar Soloist**. O Soloist recebe a chave como argumento de arranque, acessível a processos com permissões suficientes.

## Escolher o que ouvir

- **Explorar.** Cola um link direto de faixa do Spotify ou um URI `spotify:track:…` e escolhe **Ouvir link**. O SpotifyRS abre a faixa no Soloist.
- **Fila local.** Vê até dez faixas vindouras e escolhe **Ouvir** junto à que queres iniciar. **Atualizar fila** volta a consultar o Soloist.
- **Pesquisa.** Regista `http://127.0.0.1:8888/callback` na tua aplicação no Spotify for Developers. Em **Ligações**, cola o **Client ID** e escolhe **Autorizar catálogo**. Conclui a autorização no navegador e volta a **Explorar**. Podes pesquisar pelo título ou artista e ouvir uma faixa a partir dos resultados. A autorização do catálogo não pede acesso a controlos de reprodução.

Usa **Ativar reprodução neste computador** em **Ligações** se o Soloist estiver emparelhado mas inativo. A barra inferior tem os controlos de reprodução. **Desligar** ou fechar a app termina o Soloist que o SpotifyRS iniciou. Se o iniciaste noutra aplicação, podes usar **Ligar Soloist local** com o endereço correspondente.

O executável Soloist não está incluído no pacote. Cada utilizador precisa da sua chave e de obter as versões oficiais; as builds expiram após 90 dias. Não expor a porta à rede: a API local não tem autenticação. O companion rejeita endereços que não sejam loopback.

## Estado da validação

Os testes usam servidores simulados para verificar a pesquisa e o envio da faixa escolhida. A reprodução real e o consumo conjunto SpotifyRS + Soloist ainda dependem da instalação e chave do utilizador. Os números de RAM em VALIDACAO.md pertencem a uma versão anterior, sem Soloist.

Referências: [integração oficial](https://developer.spotify.com/documentation/soloist/howtos/basic-integration) e [protocolo WebSocket](https://developer.spotify.com/documentation/soloist/reference/websocket-api).
