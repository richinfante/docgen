# docgen
Docgen is a static site renderer which is built using servo's html5ever and spidermonkey. It aims to make static site generation to be effortless and removing templating languages such as liquid.

Docgen's syntax is based on / inspired by the syntax used by vuejs's templates.

**NOTE: Docgen is experimental software. It's not feature complete, and doesn't work half the time. It's extremely unstable, and this repo is published mostly for feedback purposes.**

## Feature Roadmap
these are features I'd like to have, in no particular order.
- [x] add html parser
- [x] add js engine
- [x] conditional logic with `x-if` (tentative name)
- [x] attribute variable expansion (`:href="link.title"` with `link = { title: 'HI' }` -> `href="HI"`)
- [ ] iteration logic with `x-each` (tentative name) **Experimentally Implemented**
- [ ] conditional css class generation (similar to vuejs's :class attribute).
- [ ] json/yaml/etc data file loading for configuration / data.
- [ ] markdown support with front-matter data + rendering (similar to jekyll)
- [ ] page-fork rendering: instead of iterating a page via `x-each`, render multiple copies of a page with different elements. To be used for dynamic tagging.

## Build
```bash
brew install yasm
set AUTOCONF="$(which autoconf)"
cargo build
```

## Run
```bash
cat demo.html | cargo run
```

## Template Examples
At the moment, docgen only produces processes pure html templates. This will change in the future, with options for markdown, etc.

### Source Template
```
<html>
<head>
  <script ssr="true">
  x = 1337;
  let SECRET_ENABLED = true;
  let links = [{
    href: 'https://google.com',
    title: 'google'
  }, {
    href: 'https://apple.com',
    title: 'apple'
  },{
    href: 'https://amazon.com',
    title: 'amazon'
  }]
  </script>
</head>
<body>
  <ul>
    <!-- TODO: allow binding to different variables. -->
    <!-- Currently, only binds to "item" which makes nesting not work -->
    <li x-each="links">
      <a :href="item.href">{{item.title}}</a>
    </li>
  </ul>
</body>
</html>
```

### Result Output
```
<html>
<head>
  <!-- TODO: remove ssr scripts when rendering -->
  <script ssr="true">
  let links = [{
    href: 'https://google.com',
    title: 'google'
  }, {
    href: 'https://apple.com',
    title: 'apple'
  },{
    href: 'https://amazon.com',
    title: 'amazon'
  }]
  </script>
</head>
<body>
  <ul>
    <!-- TODO: remove iteration attributes -->
    <li x-each="links">
      <a href="https://google.com">google</a>
    </li><li x-each="links">
      <a href="https://apple.com">apple</a>
    </li><li x-each="links">
      <a href="https://amazon.com">amazon</a>
    </li>
  </ul> 
</body>
</html>
```
