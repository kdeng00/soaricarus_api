# soaricarus_api
Core web API for the soaricarus project.


### Requires
`soaricarus_auth` v0.8.x  
`songparser` v0.6.x

### Compatible with
`sidm` v0.10.x  


## Getting Started
Quickest way to get started is with docker. Make sure `soaricarus_auth` and `songparser` repositories 
are located in the root of the parent directory. Check the respective repositories to ensure they
are setup correctly before configuring `soaricarus_api`.

Copy the `.env.docker.sample` file to `.env`. Ensure that the `ROOT_DIRECTORY` variable is populated
and exists on the docker image's filesystem. The credentials for the database doesn't need to be
changed for development, but if deploying it, it should be modified.

### S3 Setup
Using the copied `.env.docker.sample`, ensure that the following has been properly populated.
```
S3_ENDPOINT_URL - URL of the S3 bucket
S3_BUCKET_NAME - Name of the S3 bucket
GARAGE_S3_REGION - Region of the S3 bucket
GARAGE_RPC_SECRET - A 32-bit hex string
GARAGE_DEFAULT_ACCESS_KEY - Access key starting with GK followed with a 16-bit hex string
GARAGE_DEFAULT_SECRET_KEY - Secret key with a 32-bit hex string
```

The other S3-related environment variables can be left as is for the sake of getting
started.


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
