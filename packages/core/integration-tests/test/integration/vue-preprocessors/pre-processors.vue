<style lang="scss">
$red: red;
h1 {
  background: $red;
}
</style>

<style lang="less">
@green: green;
h1 {
  color: @green;
}
</style>

<script lang="coffee">
module.exports =
  data: ->
    msg: 'Hello from coffee!'
</script>
