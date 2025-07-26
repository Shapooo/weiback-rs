<template>
  <div id="app">
    <nav>
      <router-link to="/online-backup">在线备份</router-link>
      <router-link to="/export">本地导出</router-link>
      <router-link to="/settings">设 置</router-link>
      <router-link to="/user">用 户</router-link>
    </nav>
    <router-view></router-view>
  </div>
</template>

<style>
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: #2c3e50;
}

nav {
  padding: 30px;
}

nav a {
  font-weight: bold;
  color: #2c3e50;
  margin: 0 15px;
}

nav a.router-link-exact-active {
  color: #42b983;
}
</style>
