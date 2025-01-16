<script setup>
defineProps({
  time: {
    type: Object,
    required: true,
  },
});
</script>

<script>
// export default {
//     data(){},
//     methods:{
// change_type_toggle(ev){
//     time.change={ev.target.value: 0};
//     return true;
// };
// },
// };
// @change="ev => time.change={ev.target.value:0} "
</script>

<template>
    <div class="grid">
        <select name="time_type">
            <option value="Time">Time</option>
            <option value="Sunrise">Sunrise</option>
            <option value="SunriseOffset">SunriseOffset</option>
            <option value="Sunset">Sunset</option>
            <option value="SunsetOffset">SunsetOffset</option>
        </select>
        <input type="number" min="0" max="23" :value="23" />
        <input type="number" min="0" max="59" :value="59" />
        <select name="change_type">
            <option value="Brightness">Brightness</option>
            <option value="Preset">Preset</option>
        </select>
        <input v-if="'Brightness' in time.change" type="number" min="0.0" max="1.0" step=".1" :value="time.change.Brightness"/>
        <input v-if="'Preset' in time.change" type="number" min="0" max="255" step="1" :value="time.change.Preset" />


    </div>
</template>

<style scoped>
</style>
