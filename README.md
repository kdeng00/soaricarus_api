# Icarus
Web API for the Icarus project.


### Requires
`icarus_auth` v0.8.x  
`songparser` v0.6.x

### Compatible with
`icarus-dm` v0.9.x  


## Getting Started
Quickest way to get started is with docker. Make sure `icarus_auth` and `songparser` repositories 
are located in the root of the parent directory. Check the respective repositories to ensure they
are setup correctly before configuring `Icarus`.

Copy the `.env.docker.sample` file to `.env`. Ensure that the `ROOT_DIRECTORY` variable is populated
and exists on the docker image's filesystem. The credentials for the database doesn't need to be
changed for development, but if deploying it, it should be modified.

Build containers
```
docker compose build --ssh default
```

Bring it up
```
docker compose up -d --force-recreate
```

To view the OpenAPI spec, run the project and access `/swagger-ui`. If running through docker,
the url would be something like `http://localhost:8000/swagger-ui`.
