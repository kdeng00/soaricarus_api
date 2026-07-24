# FROM dxflrs/garage:v2.3.0
# Stage 1: Extract the garage binary from the official image
FROM dxflrs/garage:v2.3.0 as garage-base

# Stage 2: Create a new image with a shell
FROM alpine:latest

# Install necessary tools (curl, busybox for sh, etc.)
# The Garage image is based on Alpine, so we use apk
RUN apk add --no-cache curl busybox

# Copy the garage binary from the official image
COPY --from=garage-base /garage /usr/local/bin/garage

# Copy the init script
COPY init-garage.sh /init-garage.sh
RUN chmod +x /init-garage.sh

# Use the init script as the entrypoint
# It will run setup, then exec garage
ENTRYPOINT ["/init-garage.sh"]
