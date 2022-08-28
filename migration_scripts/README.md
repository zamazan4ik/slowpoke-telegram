Migration steps:
1. run `git fetch && git merge eeceecd97eb00c45330bb219deffb83abd5e5c74`
2. if you are not using docker, then build sources and run `python3 ./migration_scripts/migration_script.py`. Otherwise run docker container
3. run `git fetch && git merge 80b5e1d1827cfd234479fa4429b7263f827f17d5`
4. if you are not using docker, then build sources and run bot. Otherwise run docker container

Revert migration steps:
1. run `python3 ./migration_scripts/revert_migration_script.py`. Don't forget to roll back the changes in the code