<script setup>
import { ref } from 'vue';
import { CYBP_backend } from 'declarations/CYBP_backend/index';

let priced = ref('');

async function handleSubmit(e) {
  e.preventDefault();
  const target = e.target;
  const datat = target.querySelector('#datat').value;
  
  // Basic validation for date format (YYYYMMDD)
  if (!/^\d{8}$/.test(datat)) {
    priced.value = 'Invalid date format. Use YYYYMMDD.';
    return;
  }

  try {
    // Call the backend function with the argument
    const response = await CYBP_backend.get_icp_usd_prices(datat ? [datat] : []);


    // Destructure the response to get the predicted price
    const result = response.Ok || response.Err;
    if (result) {
      priced.value = response.Ok ? response.Ok.toString() : response.Err;
    } else {
      priced.value = 'Price not available';
    }
  } catch (error) {
    console.error('Error fetching price:', error);
    priced.value = 'Failed to fetch price';
    errorMsg.value = error.message;
  }
}
</script>




<template>
  <form @submit="handleSubmit">
    <input type="text" id="datat" placeholder="Enter date (YYYYMMDD)" />
    <button type="submit">Get ICP Price</button>
  </form>
  <p>Predicted Price: {{ priced }}</p>
</template>

