<script setup>
import TimeSetter from './components/timesetter.vue'
</script>

<script>

export default {
  data(){
    return {
            count:0,
            config: {},
            timeformat: "hh:mm a",
            timedata: "12:45 AM",
    }
  },
  components: {
  },
  created(){
    console.log("Time to kick of retrieval of our WLED components.");
    this.fetchData();
  },
  methods:{
    async fetchData() {
      const url = `json/config`
      this.config = await (await fetch(url)).json()
      this.config = ref(this.config)
    },
    markDirty(wled, dirty){
      this.config.leds[wled].dirty=dirty;
      console.log("Marked led ", wled, " dirty? ", dirty);
      return true
    }
    

  },

}
</script>

<template>

  <main class="container">
    <div id="app">
      <details open>
      <summary>Devices</summary>
      <p v-if="config.leds && Object.keys(config.leds) && Object.keys(config.leds).length == 0">
      You haven't added any WLEDs yet.
      </p>
      <table v-if="config.leds && Object.keys(config.leds).length > 0" class="striped">
        <tr><th><b>Name</b></th><th>Min Bri</th><th>Max Bri</th><th>Schedule</th><th>Actions</th></tr>
        <tr v-for="wled in Object.keys(config.leds).sort()" :class='config.leds[wled].dirty?"dirty":""'>
            <td><b>{{wled.split('.')[0]}}</b></td>
            <td><input type="number" min="0" max="255" :value="config.leds[wled].min_bri" @change="ev => {markDirty(wled, true); config.leds[wled].min_bri = ev.target.value}"></td>
            <td><input type="number" min="0" max="255" :value="config.leds[wled].max_bri" @change="markDirty(wled, true)"></td>
            <td>
                <select :value="config.leds[wled].schedule.ByName||config.leds[wled].schedule">
                    <option value="None">No Schedule</option>
                    <option value="Default">Default</option>
                    <option v-if="config.schedule" v-for="schedule in Object.keys(config.schedule)" >{{ schedule }}</option>
                </select>
            </td>
            <td><button>Save</button></td>
        </tr>
      </table>
      <p v-if="config.schedule && Object.keys(config.schedule).length == 0">
      No schedules created yet.
      </p>
      </details>
      <hr/>
      <details>
      <summary>Schedules</summary>
      <div v-if="config.schedule && Object.keys(config.schedule).length > 0">
        <div v-for="schedulename in Object.keys(config.schedule)">
          <details><summary>{{ schedulename }}</summary>
          <article>

            <div v-for="schedule_item in config.schedule[schedulename].keys()">
              <TimeSetter :time="config.schedule[schedulename][schedule_item]" />
            </div>
            <button>+</button>
          </article>
          </details>
        </div>
      </div>
      </details>


    </div>
  </main>

</template>

<style scoped>
header {
  line-height: 1.5;
}

tr.dirty{
    background: #ffaaaa;
}

.logo {
  display: block;
  margin: 0 auto 2rem;
}

@media (min-width: 1024px) {
  header {
    display: flex;
    place-items: center;
    padding-right: calc(var(--section-gap) / 2);
  }

  .logo {
    margin: 0 2rem 0 0;
  }

  header .wrapper {
    display: flex;
    place-items: flex-start;
    flex-wrap: wrap;
  }
}
</style>
