#define YA_ECGEN_ERROR_COOL(message) static_assert(false, message)

#define YA_ECGEN_ERROR(message) static_assert(false, message)
#define YA_ECGEN_ERROR_MESSAGE_UNPARITY "YA_ECGEN_: [Argument unparity] Error code doesn't have its message pair."
#define YA_ECGEN_ERROR_MESSAGE_NO_ARGS "YA_ECGEN_: [No members] No member was specified for this enum type."

#define YA_ECGEN___ARGS__0_0(lowercase_name, UPPERCASE_NAME) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_NO_ARGS)
#define YA_ECGEN___ARGS__0_1(lowercase_name, UPPERCASE_NAME, __0__) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_UNPARITY)
#define YA_ECGEN___ARGS__0_2(lowercase_name, UPPERCASE_NAME, __0__, __1__) YA_ ## UPPERCASE_NAME ## _ ## __0__
#define YA_ECGEN___ARGS__0_3(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_UNPARITY)
#define YA_ECGEN___ARGS__0_4(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__) YA_ ## UPPERCASE_NAME ## _ ## __0__,YA_ ## UPPERCASE_NAME ## _ ## __2__
#define YA_ECGEN___ARGS__0_5(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__, __4__) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_UNPARITY)
#define YA_ECGEN___ARGS__0_6(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__, __4__, __5__) YA_ ## UPPERCASE_NAME ## _ ## __0__,YA_ ## UPPERCASE_NAME ## _ ## __2__,YA_ ## UPPERCASE_NAME ## _ ## __4__
#define YA_ECGEN___ARGS__0_7(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__, __4__, __5__, __6__) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_UNPARITY)
#define YA_ECGEN___ARGS__0_8(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__, __4__, __5__, __6__, __7__) YA_ ## UPPERCASE_NAME ## _ ## __0__,YA_ ## UPPERCASE_NAME ## _ ## __2__,YA_ ## UPPERCASE_NAME ## _ ## __4__,YA_ ## UPPERCASE_NAME ## _ ## __6__
#define YA_ECGEN___ARGS__0_9(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__, __4__, __5__, __6__, __7__, __8__) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_UNPARITY)
#define YA_ECGEN___ARGS__0(__0__, __1__, __2__, __3__, __4__, __5__, __6__, __7__, __8__, __9__, __NAME__, ...) __NAME__
#define YA_ECGEN___ARGS__1_0(lowercase_name, UPPERCASE_NAME) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_NO_ARGS)
#define YA_ECGEN___ARGS__1_1(lowercase_name, UPPERCASE_NAME, __0__) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_UNPARITY)
#define YA_ECGEN___ARGS__1_2(lowercase_name, UPPERCASE_NAME, __0__, __1__) [YA_ ## UPPERCASE_NAME ## _ ## __0__] = __1__
#define YA_ECGEN___ARGS__1_3(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_UNPARITY)
#define YA_ECGEN___ARGS__1_4(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__) [YA_ ## UPPERCASE_NAME ## _ ## __0__] = __1__, [YA_ ## UPPERCASE_NAME ## _ ## __2__] = __3__
#define YA_ECGEN___ARGS__1_5(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__, __4__) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_UNPARITY)
#define YA_ECGEN___ARGS__1_6(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__, __4__, __5__) [YA_ ## UPPERCASE_NAME ## _ ## __0__] = __1__, [YA_ ## UPPERCASE_NAME ## _ ## __2__] = __3__, [YA_ ## UPPERCASE_NAME ## _ ## __4__] = __5__
#define YA_ECGEN___ARGS__1_7(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__, __4__, __5__, __6__) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_UNPARITY)
#define YA_ECGEN___ARGS__1_8(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__, __4__, __5__, __6__, __7__) [YA_ ## UPPERCASE_NAME ## _ ## __0__] = __1__, [YA_ ## UPPERCASE_NAME ## _ ## __2__] = __3__, [YA_ ## UPPERCASE_NAME ## _ ## __4__] = __5__, [YA_ ## UPPERCASE_NAME ## _ ## __6__] = __7__
#define YA_ECGEN___ARGS__1_9(lowercase_name, UPPERCASE_NAME, __0__, __1__, __2__, __3__, __4__, __5__, __6__, __7__, __8__) YA_ECGEN_ERROR(YA_ECGEN_ERROR_MESSAGE_UNPARITY)
#define YA_ECGEN___ARGS__1(__0__, __1__, __2__, __3__, __4__, __5__, __6__, __7__, __8__, __9__, __NAME__, ...) __NAME__
#define YA_ECGEN___GENERATOR__0(lowercase_name, UPPERCASE_NAME, __GEN__, ...) enum ya_ ## lowercase_name ## _error_codes {__GEN__(lowercase_name, UPPERCASE_NAME, __VA_ARGS__)};

#define YA_ECGEN___GENERATOR__1(lowercase_name, UPPERCASE_NAME, __GEN__, ...) const char *ya_ ## lowercase_name ## _conversion_table[] = {__GEN__(lowercase_name, UPPERCASE_NAME, __VA_ARGS__)};

#define YA_ECGEN(lowercase_name, UPPERCASE_NAME, ...) YA_ECGEN___GENERATOR__0(lowercase_name, UPPERCASE_NAME, YA_ECGEN___ARGS__0("empty", ##__VA_ARGS__, YA_ECGEN___ARGS__0_9, YA_ECGEN___ARGS__0_8, YA_ECGEN___ARGS__0_7, YA_ECGEN___ARGS__0_6, YA_ECGEN___ARGS__0_5, YA_ECGEN___ARGS__0_4, YA_ECGEN___ARGS__0_3, YA_ECGEN___ARGS__0_2, YA_ECGEN___ARGS__0_1, YA_ECGEN___ARGS__0_0), __VA_ARGS__) YA_ECGEN___GENERATOR__1(lowercase_name, UPPERCASE_NAME, YA_ECGEN___ARGS__1("empty", ##__VA_ARGS__, YA_ECGEN___ARGS__1_9, YA_ECGEN___ARGS__1_8, YA_ECGEN___ARGS__1_7, YA_ECGEN___ARGS__1_6, YA_ECGEN___ARGS__1_5, YA_ECGEN___ARGS__1_4, YA_ECGEN___ARGS__1_3, YA_ECGEN___ARGS__1_2, YA_ECGEN___ARGS__1_1, YA_ECGEN___ARGS__1_0), __VA_ARGS__) 

YA_ECGEN(
    hello, HELLO,
    HI, "HI"
)