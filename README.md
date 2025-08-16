# Xi's Page

[![GitHub deployments](https://img.shields.io/github/deployments/aefhm/page/production)](https://github.com/aefhm/page/deployments)

## Why

I believe in long term content and that pages should [last](https://jeffhuang.com/designed_to_last/).

## Where

This static site is deployed to [Cloudflare Workers](https://developers.cloudflare.com/workers/) with naked domain redirect.

## How

This site follows the structure below:

- `/recipes/{recipe.html}`
- `/writings/index.html`
- `/about`

## Local Development

### Using http-server

1. Navigate to the public directory:

   ```bash
   cd public
   ```

2. Start the server using npx:

   ```bash
   npx http-server -p 8080 --cors
   ```

   Options used:
   - `-p 8080`: Runs on port 8080
   - `--cors`: Enables CORS for fonts and other resources

3. Open your browser to:

   ```
   http://localhost:8080
   ```

### Alternative: Using Python

If you don't have Node.js installed, you can use Python's built-in server:

```bash
# Python 3
python3 -m http.server 8080
```
