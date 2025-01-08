# Twine CLI

## Design ideas

```sh
twine init # create a local dir config file and store
twine ls # list strands in local dir
twine create ./strand-info.json -k ./ed25519.key # create a strand
twine commit -s 1 ./payload.json # create a tixel with payload on strand "1"
twine sync # sync with configured remote stores
twine dump bafyreislkdf # print all strand data as json (-o car) for car output
```
