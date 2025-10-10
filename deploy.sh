#!/bin/bash
# DeFarm Engines Production Deployment Script

set -e

echo "🚀 DeFarm Engines Deployment Script"
echo "===================================="

# Check if .env exists
if [ ! -f .env ]; then
    echo "❌ Error: .env file not found"
    echo "💡 Copy .env.example to .env and configure your settings:"
    echo "   cp .env.example .env"
    exit 1
fi

# Load environment variables
source .env

# Check required environment variables
REQUIRED_VARS=("JWT_SECRET" "PINATA_API_KEY" "PINATA_SECRET_KEY")
for var in "${REQUIRED_VARS[@]}"; do
    if [ -z "${!var}" ]; then
        echo "❌ Error: $var is not set in .env"
        exit 1
    fi
done

# Create necessary directories
echo "📁 Creating necessary directories..."
mkdir -p nginx/ssl
mkdir -p nginx/logs

# Generate self-signed SSL certificates if they don't exist
if [ ! -f nginx/ssl/fullchain.pem ]; then
    echo "🔐 Generating self-signed SSL certificates..."
    echo "⚠️  For production, replace with Let's Encrypt or commercial certs!"
    openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
        -keyout nginx/ssl/privkey.pem \
        -out nginx/ssl/fullchain.pem \
        -subj "/C=US/ST=State/L=City/O=DeFarm/CN=localhost"
fi

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Error: Docker is not installed"
    echo "💡 Install Docker: https://docs.docker.com/get-docker/"
    exit 1
fi

# Check if Docker Compose is installed
if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "❌ Error: Docker Compose is not installed"
    echo "💡 Install Docker Compose: https://docs.docker.com/compose/install/"
    exit 1
fi

# Stop existing containers
echo "🛑 Stopping existing containers..."
docker-compose down 2>/dev/null || true

# Build images
echo "🔨 Building Docker images..."
docker-compose build --no-cache

# Start services
echo "🚀 Starting services..."
docker-compose up -d

# Wait for services to be healthy
echo "⏳ Waiting for services to be healthy..."
sleep 10

# Check service health
echo "🏥 Checking service health..."
if docker-compose ps | grep -q "Up"; then
    echo "✅ Services are running"
else
    echo "❌ Some services failed to start"
    echo "📋 Container logs:"
    docker-compose logs --tail=50
    exit 1
fi

# Display status
echo ""
echo "✅ Deployment complete!"
echo ""
echo "📊 Service Status:"
docker-compose ps

echo ""
echo "🔗 Service URLs:"
echo "  API (HTTP):  http://localhost"
echo "  API (HTTPS): https://localhost"
echo "  Health:      http://localhost/health"
echo ""
echo "📋 Useful commands:"
echo "  View logs:        docker-compose logs -f"
echo "  Stop services:    docker-compose down"
echo "  Restart services: docker-compose restart"
echo "  View API logs:    docker-compose logs -f api"
echo "  View DB logs:     docker-compose logs -f postgres"
echo ""
echo "⚠️  Don't forget to:"
echo "  1. Replace self-signed SSL certs with production certs"
echo "  2. Update DATABASE_URL with production PostgreSQL if using external DB"
echo "  3. Configure firewall rules"
echo "  4. Set up monitoring and alerts"
echo "  5. Configure backup strategy for PostgreSQL"
