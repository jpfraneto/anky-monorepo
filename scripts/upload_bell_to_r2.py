#!/usr/bin/env python3
"""Upload anky bell to R2 for public access"""
import boto3
import os

# R2 credentials from environment
r2_account_id = os.environ.get("R2_ACCOUNT_ID")
r2_bucket = os.environ.get("R2_BUCKET_NAME", "anky")
r2_access_key = os.environ.get("R2_ACCESS_KEY_ID")
r2_secret_key = os.environ.get("R2_SECRET_ACCESS_KEY")
r2_public_url = os.environ.get("R2_PUBLIC_URL", "https://storage.anky.app")

if not all([r2_account_id, r2_access_key, r2_secret_key]):
    print("Missing R2 credentials. Check env vars:")
    print("- R2_ACCOUNT_ID")
    print("- R2_ACCESS_KEY_ID")
    print("- R2_SECRET_ACCESS_KEY")
    exit(1)

# Create S3 client
s3_client = boto3.client(
    's3',
    aws_access_key_id=r2_access_key,
    aws_secret_access_key=r2_secret_key,
    endpoint_url=f'https://{r2_account_id}.r2.cloudflarestorage.com'
)

# Upload the bell
bell_path = os.path.expanduser("~/anky/assets/sounds/anky_bell.wav")
object_key = "sounds/anky_bell.wav"

print(f"Uploading {bell_path} to R2...")
print(f"Bucket: {r2_bucket}")
print(f"Key: {object_key}")

with open(bell_path, 'rb') as f:
    s3_client.put_object(
        Bucket=r2_bucket,
        Key=object_key,
        Body=f.read(),
        ContentType='audio/wav',
        CacheControl='public, max-age=31536000, immutable'
    )

cdn_url = f"{r2_public_url}/{object_key}"
print(f"\n✓ Uploaded successfully!")
print(f"✓ Public URL: {cdn_url}")
