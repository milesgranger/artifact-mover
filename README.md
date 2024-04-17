

## artifact-mover GitHub Action

Move artifacts to and from S3 (other targets to come).

## Use:

```yaml
# Assumes you've setup some AWS credentials (env vars, profile, etc)
# And have set aws region either as env var (AWS_REGION) or as part of the profile
- uses: milesgranger/artifact-mover@main
  with:
    action: upload
    name: my-artifact
    path: ./some-path/to/my-artifact
    bucket: my-aws-s3-bucket
    profile: my-aws-profile  # optional, defaults to default or using env vars

- uses: milesgranger/artifact-mover@main
  with:
    action: download
    name: my-artifact
    path: ./output-dir
    bucket: my-aws-s3-bucket
```

See [the integration test](./.github/workflows/test.yml) for full a working example

### AWS Permissions

The permissions needed by this action are only `get-object`, `put-object`, and `list-object-v2` for a the given bucket. All of which could be further restricted to the `prefix-key` parameter which defaults to "artifacts". ie arn:aws:s3:::<bucket>/artifacts/*
