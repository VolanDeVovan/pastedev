# PasteDev

PasteDev is a lightweight pastebin written on rust and inspired by [haste](https://github.com/toptal/haste-server) and [termbin](https://github.com/solusipse/fiche).

Hosted on [paste.dev.su](https://paste.dev.su) .

> First attempt at doing something useful with rust.

## Socket usage 

You can also paste text using socket provider on 9999 port.

### Linux 

```
cat something | nc -N paste.dev.su 9999
```

### macOS

```
cat something | nc paste.dev.su 9999
```

## TODO 

- [ ] Body size limit
- [ ] Github workflow (Deploy to registry)
- [ ] Icon and preview for the site 
- [ ] Support for new databases
- [ ] Instruction for installation
- [ ] Response with raw format (possibly)
