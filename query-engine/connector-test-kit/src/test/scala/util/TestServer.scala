package util

import play.api.libs.json._
case class TestServer() extends PlayJsonExtensions {
  def query(
      query: String,
      project: Project,
      dataContains: String = ""
  ): JsValue = {
    val result = queryBinaryCLI(
      query = query,
      project = project,
    )
    result.assertSuccessfulResponse(dataContains)
    result
  }

  def queryThatMustFail(
      query: String,
      project: Project,
      errorCode: Int,
      errorCount: Int = 1,
      errorContains: String = ""
  ): JsValue = {
    val result =
      queryBinaryCLI(
        query = query,
        project = project,
      )

    // Ignore error codes for external tests (0) and containment checks ("")
    result.assertFailingResponse(0, errorCount, "")
    result
  }

  def queryBinaryCLI(query: String, project: Project) = {
    import sys.process._

    val formattedQuery = query.stripMargin.replace("\n", "")
    println(formattedQuery)
    val encoded_query = UTF8Base64.encode(formattedQuery)
    val response: String = project.isPgBouncer match {
      case true =>
        Process(Seq(EnvVars.prismaBinaryPath, "--always_force_transactions", "cli", "--execute_request", encoded_query),
                None,
                "PRISMA_DML" -> project.pgBouncerEnvVar).!!
      case false => Process(Seq(EnvVars.prismaBinaryPath, "cli", "--execute_request", encoded_query), None, "PRISMA_DML" -> project.envVar).!!
    }
    val lines           = response.linesIterator.toVector
    val normalLogOutput = lines.init
    val lastLine        = lines.last
    println(normalLogOutput.mkString("\n"))
    val decoded_response = UTF8Base64.decode(lastLine)
    println(decoded_response)
    Json.parse(decoded_response)
  }
}
