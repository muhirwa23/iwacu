<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta http-equiv="X-UA-Compatible" content="IE=edge">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>shopy</title>
    <link rel="stylesheet" href="store.css">
    <link rel="stylesheet" href='https://unpkg.com/boxicons@2.1.1/css/boxicons.min.css'>
</head>
<body>
    <!--header-->
    <header>
        <!--nav-->
        <div class=" nav container">
            <a href="# " class="logo">eHanga</a>
            <!--icons header-->
            <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABgAAAAYCAYAAADgdz34AAAAAXNSR0IArs4c6QAAARJJREFUSEvd1M8xREEQBvDfRoAMiAARCAER4MqBEIiAC1dEgAyEQARkgAioT3lbr3hvZ97WbtWWPs709Penu2dkzjGac33/C+BzhnY9YTP12hbNEuADy78BGgEXOMYt9geqOsF5+21Xk1fxgnesDAR4xBYOcNOnIOfxcL2dWAEUS95+8kIsBHvHtJH6gJ2K4klJ3h2esdG86duDTjYFoFiyhzOclgByf4/tSvbttIxnLP6OSZucCboeAJDRzASO2ZcAcp9GLWENrwPAxqmlv6jxNXbtzgMgOxE/o6Im/hAuKWjGL2ObBSrFVAClohPvaxSkwCUOcYWjVsW+8+omN4ntn7bvB+4kuzAKpu5DrYLFBfgCoYYtGZGfX0sAAAAASUVORK5CYII="/>
         <iv class="cart-icon" id="cart-icon"></i>
         <!--cart-->
         <div class="cart">
         <h2 class="cart-title">your cart </h2>   
         <!--content-->
         <div class="cart-content">
            <div class="cart-box">
              
            </div>
         </div>
<!--total-->
<div class="total">
    <div class="total-title">total</div>
    <div class="total-price"> $ 0 </div>
</div>
<!--buy button-->
<button type="button" class="btn-buy"> Buy now </button>
<!--cart close-->
<i class="bx bx-x" id="close-cart"></i>
         </div>
        </div>
    </header>

    <section class="shop container">
        <h2 class="section-title">shop Haha</h2>
        <!--content-->
        <div class="shop-content">
            <!--box 1-->
            <div class="product-box">
                <img src="a1.jpg" alt="" class="product-img">
                <h2 class="product-title"> swatx</h2>
                <span class="price">$12</span>
                <i class="bx bx-shopping-bg add-cart"></i>
            </div>
             <!--box 1-->
             <div class="product-box">
                <img src="a4.jpg" alt="" class="product-img">
                <h2 class="product-title"> paritno</h2>
                <span class="price">$20</span>
                <i class="bx bx-shopping-bg add-cart"></i>
            </div>
             <!--box 1-->
             <div class="product-box">
                <img src="a5.jpg" alt="" class="product-img">
                <h2 class="product-title"> Gibusat</h2>
                <span class="price">$40</span>
                <i class="bx bx-shopping-bg add-cart"></i>
            </div> <!--box 1-->
            <div class="product-box">
                <img src="a6.jpg" alt="" class="product-img">
                <h2 class="product-title"> Gibxt</h2>
                <span class="price">$12</span>
                <i class="bx bx-shopping-bg add-cart"></i>
            </div> <!--box 1-->
            <div class="product-box">
                <img src="a8.jpg" alt="" class="product-img">
                <h2 class="product-title"> kotier</h2>
                <span class="price">$15</span>
                <i class="bx bx-shopping-bg add-cart"></i>
            </div> <!--box 1-->
            <div class="product-box">
                <img src="b2.jpg" alt="" class="product-img">
                <h2 class="product-title"> Bag </h2>
                <span class="price">$12</span>
                <i  class="bx bx-shopping-bg add-cart"></i>
            </div>
             <!--box 1-->
             <div class="product-box">
                <img src="b8.jpg" alt="" class="product-img">
                <h2 class="product-title"> chip</h2>
                <span class="price">$71</span>
                <i class="bx bx-shopping-bg add-cart"></i>
            </div> <!--box 1-->
            <div class="product-box">
                <img src="b10.jpg" alt="" class="product-img">
                <h2 class="product-title"> Houstyo</h2>
                <span class="price">$19</span>
                <i class="bx bx-shopping-bg add-cart"></i>
            </div>
        </div>
    </section>
    <!--end point-->
    <section class="footer mt-5">
        <div class="container mt-5">
            <div class="row text-center mt-5">
                <p>© eHanga 2022</inc></p>
            </div>
        </div>
    </section>
    @import url('https://fonts.googleapis.com/css2?family=poppins:wght@400;500;600;700&display=swap');
*{
    font-family: 'poppins', sans-serif;
    margin: 0;
    padding: 0;
    scroll: padding top 2rem; ;
    scroll-behavior: smooth;
    box-shadow: border-box;
    list-style: none;
    text-decoration: none;
}
:root{
    --main-color:rgba(17, 150, 50, 0.75);
    --text-color:#171427;
    --bg-color:#ffd2;
    --bgu-color:rgba(249, 27, 27, 0.936);
}
img{
    width: 100%;
}
body{
    color: var(--text-color);
    
}
.container{
    max-width: 1068px;
    margin: auto;
    width: 100%;
}
section{
    padding: 4rem 0 3rem;
}

header{
    position: fixed;
     top: 0;
     left: 0;
     width: 100%;
     background-color: #ffd2;
     box-shadow: 0 1px 4px hsl(0 4% 15% /100%);
    z-index: 100;
}
.nav{
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 0;
}
.logo{
    font-size: 1.1rem;
    color: var(--text-color);
    font-weight: 400;
}
#cart-icon{
font-size: 1.8rem;
cursor: pointer;
}
/*cart*/
.cart{
    position: fixed;
    top: 0;
    right: 0;
    width: 360px;
    min-height: 100vh;
    padding: 20px;
    background: var(--main-color);
    box-shadow: -2px 0 4 rgba(31, 30, 30, 0.1);
    transition: 0.3s;
}
.cart.active{
    right: 12;
}
.cart-title{
    text-align: center;
    font-size: 1.5rem;
    font-weight: 900;
    margin-top: 2rem;
}
.cart-box{
    display: grid;
    grid-template-columns: 32% 50% 18% ;
    align-items: center;
    gap: 1rem;
    margin-top: 1rem;
}
.cart-img{
    width: 100px;
    height: 100px;
    object-fit: contain;
    padding: 10px;
}
.detail-box{
    display: grid;
    row-gap: 0.5rem;
}
.cart-product-title{
    font-size: 1rem;
    text-transform: uppercase;
}
.cart-price{
    font-weight: 500;
}
.cart-quantity{
    border: 1px solid var(--text-color);
    outline-color: var(--main-color);
    width: 4.2rem;
    text-align: center;
    font-size: 1rem;
}

.cart-remove{
    font-size: 24px;
    cursor: pointer;
    background-color: var(--bgu-color);
}
.total{
    display: flex;
    justify-content: flex-end;
    margin-top: 1.5rem;
    border-top: 1px solid var(--text-color);
}
.total-title{
    font-size: 1rem;
    font-weight: 600;

}
.total-price{
    margin-left: 0.5rem;

}
.btn-buy{
    display: flex;
    margin: 1.5rem auto 0 auto;
    padding: 12px 20px;
    border: none;
    background:var(--bgu-color) ;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
}
.btn-buy:hover{
    background: var(--main-color);
}
#close-cart{
    position: absolute;
    top: 1rem;
    right: 0.8rem;
    font-size: 2rem;
    color: var(--text-color);
    cursor: pointer;
}
.section-title{
    font-size: 1.5rem;
    font-weight: 600;
    text-align: center;
    margin-bottom: 1.5rem;
}
.shop{
    margin-top: 2rem;
}
.shop-content{
    display: grid;
    grid-template-columns: repeat(auto-fit,minmax(220px,auto));
    gap: 1.5rem;
}
.product-box{
  position: relative;

}
.product-box:hover{
  padding: 10px;
  border: 1px solid var(--text-color);
  transition: 0.4s;
}
.product-img{
  width: 100%;
  height: auto;
  margin-bottom: 0.5rem;
}
.product-title{
  font-size: 1.1rem;
  font-weight: 700;
  text-transform: uppercase;
  margin-bottom: 0.5rem;
}
.price{
  font-weight: 700;
}
img{
    width: 12px;
}
.add-cart{
  position: absolute;
  bottom: 0;
  right: 0;
  background: var(--text-color);
  color: var(--bg-color);
  padding: 10px;
  cursor: pointer;
  width: 20px;
}.shop-content{
    display: grid;
    grid-template-columns: repeat(auto-fit,minmax(220px,auto));
    gap: 1.5rem;
}
.product-box{
  position: relative;

}
.product-box:hover{
  padding: 10px;
  border: 1px solid var(--text-color);
  transition: 0.4s;
}
.product-img{
  width: 100%;
  height: auto;
  margin-bottom: 0.5rem;
}
.product-title{
  font-size: 1.1rem;
  font-weight: 700;
  text-transform: uppercase;
  margin-bottom: 0.5rem;
}
.price{
  font-weight: 700;
}
.add-cart{
  position: absolute;
  bottom: 0;
  right: 0;
  background: var(--text-color);
  color: var(--bg-color);
  padding: 10px;
  cursor: pointer;
}
.shop-content{
      display: grid;
      grid-template-columns: repeat(auto-fit,minmax(220px,auto));
      gap: 1.5rem;
}
.product-box{
    position: relative;

}
.product-box:hover{
    padding: 10px;
    border: 1px solid var(--text-color);
    transition: 0.4s;
}
.product-img{
    width: 100%;
    height: auto;
    margin-bottom: 0.5rem;
}
.product-title{
    font-size: 1.1rem;
    font-weight: 700;
    text-transform: uppercase;
    margin-bottom: 0.5rem;
}
.price{
    font-weight: 700;
}
.add-cart{
    position: absolute;
    bottom: 0;
    right: 0;
    background: var(--text-color);
    color: var(--bg-color);
    padding: 10px;
    cursor: pointer;
}
.add-cart:hover{
    background: hsl(249,32%,17%);

}
//cart
let cartIcon= document.querySelector('.cart-icon');
let cart = document.querySelector('.cart');
let closeCart = document.querySelector('# close-cart');
//open cart
cartIcon.onclick = () =>{
    cart.classList.add( "active" )
}
//close cart
closeCart.onclick = () =>{
    cart.classList.remove( "active" )
}
// cart working js
if ( document.readyState == 'loading'){
    document.addEventListener('DOMContentLoaded',ready)
}else {
    ready();
}
//making function
function ready() {
//reomve items from cart
var reomvecartbuttons = document.getElementsByClassName('cart-remove');
console.log(reomvecartbuttons)
for ( var i=0 ; i <reomvecartbuttons.length ; i ++) {
    var button =reomvecartbuttons[i]
    button.addEventListener("click" ,removecartItem)
}
//quantinty changes
var quantintyInputs =document.getElementsByClassName('cart-quantity')
for ( var i=0 ; i <quantintyInputs.length ; i ++){
    var input =quantintyInputs[i]
    input.addEventListener('change', quantitychanged);
}
//add cart
var addCart =document.getElementsByClassName('add-cart')
for ( var i=0 ; i <addCart.length ; i ++){
    var button =addCart[i]
    button.addEventListener('click',addCartclicked) ;
}
}
// remove cart items
function removecartItem(event){
    var buttonclicked =event.target
    buttonclicked.parentelement.remove();
    updatetotal ();
}
// quanttity changes
function quantitychanged(event){
    var input =event.target
if(isNaN(input.value)|| input.value<=0){
    input.value=1 ;
}
updatetotal() ;
}
//add to cart
function addCartclicked(event){
    var button =event.target
    var shopproducts = button.parentelement
    var title =shopproducts.getElementsByClassName('product-title')[0].innertext;
    var price =shopproducts.getElementsByClassName('price')[0].innertext;
    var productImg =shopproducts.getElementsByClassName("product-img")[0].src;
    addproductTocart.log(title,price,productImg);
    updatetotal();
}
function addproductTocart(title,price,productImg){
    var cartShopbox =document.createElement("div");
    cartShopbox.classList.add('cart-box')
    var cartItems = document.getElementsByClassName('cart-content')[0];
    var cartItemsNames= cartItems.getElementsByClassName('cart-product-title');
    for ( var i=0 ; i <cartItemsNames.length ; i ++){
    alert("you have already add  items to cat");
    return;
    }
} 
var cartBoxcontent =`
      <img src="a4.jpg" alt="" class="cart-img">
      <div class="detail-box">
      <div class="cart-product-title">cds23</div>
     <div class="cart-price">$32</div>
     <input type="number" value="1" class="cart-quantity">
     </div>
     <!--remove cart-->
     <img src="images.png" class="bx bxs-trash-alt cart-remove" >`;
cartShopbox.innerHTML=cartBoxcontent
cartItem.append(cartShopbox)
cartShopbox.getElementsByClassName('cart-remove')[0].addEventListener("click",removecartItem)
cartShopbox.getElementsByClassName('cart-quantity')[0].addEventListener("change",quantitychanged)

//update total
function updatetotal(){
    var cartContent = Document.getElementsByClassName('cart-content')[0];
    var cartBoxes =cartContent.getElementsByClassName('cart-box')
    for ( var i=0 ; i <cartBoxes.length ; i ++){
        var total =0
var cartBox=cartBoxes[i]
var priceElement =cartBox.getElementsByClassName('cart-price')[0];
var quantintyElement = cartBox.getElementsByClassName('cart-quantity')[0];

var price =parseFloat(priceElement.innertext.replace("$" ,"")) ;
var quantity = quantintyElement.value
total = total+ (price*quantity);
//if price contain same cents value
total =Math.round(total*100)/100 ;
document.getElementsByClassName('total-price')[0].innertext = '$'+total;
    }
}

</body>
</html>
