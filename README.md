# docgen

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
