function onchangeRegexRadio(target) {
    const form = document.querySelector("#RegexInputForm");
    const input = form.querySelector("#RegexInput");
    if (target.value === "UnknownRegex"){
        form.hidden = true;
        input.required = false;
        if (input.name){
            input.removeAttribute("name");
        }
    }else {
        form.hidden = false;
        input.required = true;
        input.setAttribute("name", "regex");
    }
}

document.querySelector(".radio-regex>input[type='radio']:checked").click();

